//! Adds Relay's Browser Sources to OBS through obs-websocket 5 (built into OBS 28+).
//! Only 127.0.0.1 is contacted; the password comes from Windows Credential Manager.

use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, protocol::frame::coding::CloseCode},
};

pub const DEFAULT_OBS_PORT: u16 = 4455;
const STEP_TIMEOUT: Duration = Duration::from_secs(5);
const RPC_VERSION: u64 = 1;
/// obs-websocket closes with 4009 when authentication fails.
const AUTHENTICATION_FAILED: u16 = 4009;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsScenes {
    pub scenes: Vec<String>,
    pub current: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSource {
    pub name: String,
    /// `created`, `updated` or `added` (existing source placed in the scene).
    pub action: &'static str,
}

/// One Browser Source Relay manages in OBS.
#[derive(Clone, Debug)]
pub struct SourceSpec {
    pub name: &'static str,
    pub url: String,
}

pub fn relay_sources(port: u16, include_reactions: bool) -> Vec<SourceSpec> {
    let mut sources = vec![
        SourceSpec {
            name: "Relay Visual",
            url: crate::widget::obs_visual_url(port),
        },
        SourceSpec {
            name: "Relay Audio",
            url: format!("http://127.0.0.1:{port}/obs/audio"),
        },
    ];
    if include_reactions {
        sources.push(SourceSpec {
            name: "Relay Reactions",
            url: format!("http://127.0.0.1:{port}/reactions"),
        });
    }
    sources
}

/// Settings shared by every Relay source: full-canvas, transparent, audio routed to the OBS mixer.
pub fn browser_settings(url: &str) -> Value {
    json!({
        "url": url,
        "is_local_file": false,
        "width": 1920,
        "height": 1080,
        "reroute_audio": true,
        "shutdown": false,
        "restart_when_active": false,
        "css": "",
    })
}

/// obs-websocket authentication string: base64(sha256(base64(sha256(password + salt)) + challenge)).
pub fn authentication(password: &str, salt: &str, challenge: &str) -> String {
    let secret = STANDARD.encode(Sha256::digest(format!("{password}{salt}")));
    STANDARD.encode(Sha256::digest(format!("{secret}{challenge}")))
}

struct ObsClient {
    socket: Socket,
    next_id: u64,
}

impl ObsClient {
    async fn connect(port: u16, password: Option<&str>) -> Result<Self> {
        let url = format!("ws://127.0.0.1:{port}");
        let (socket, _) = timeout(STEP_TIMEOUT, connect_async(url.as_str()))
            .await
            .map_err(|_| anyhow!("OBS did not answer in time."))?
            .map_err(|_| {
                anyhow!(
                    "OBS is not reachable. In OBS, open Tools → WebSocket Server Settings and enable the WebSocket server."
                )
            })?;
        let mut client = Self { socket, next_id: 1 };
        let hello = client.receive_op(0).await?;
        let mut identify = json!({ "rpcVersion": RPC_VERSION, "eventSubscriptions": 0 });
        if let Some(auth) = hello.get("authentication") {
            let password = password.filter(|value| !value.is_empty()).ok_or_else(|| {
                anyhow!("OBS asks for a WebSocket password. Copy it from Tools → WebSocket Server Settings → Show Connect Info.")
            })?;
            let salt = auth["salt"]
                .as_str()
                .context("OBS sent an invalid challenge.")?;
            let challenge = auth["challenge"]
                .as_str()
                .context("OBS sent an invalid challenge.")?;
            identify["authentication"] = json!(authentication(password, salt, challenge));
        }
        client.send(json!({ "op": 1, "d": identify })).await?;
        client.receive_op(2).await?;
        Ok(client)
    }

    async fn send(&mut self, value: Value) -> Result<()> {
        self.socket
            .send(Message::text(value.to_string()))
            .await
            .map_err(|_| anyhow!("The connection to OBS was lost."))
    }

    async fn receive_op(&mut self, op: u64) -> Result<Value> {
        loop {
            let message = timeout(STEP_TIMEOUT, self.socket.next())
                .await
                .map_err(|_| anyhow!("OBS did not answer in time."))?
                .ok_or_else(|| anyhow!("The connection to OBS was lost."))?
                .map_err(|_| anyhow!("The connection to OBS was lost."))?;
            match message {
                Message::Text(text) => {
                    let value: Value = serde_json::from_str(text.as_str())
                        .context("OBS sent an invalid message.")?;
                    if value["op"].as_u64() == Some(op) {
                        return Ok(value["d"].clone());
                    }
                }
                Message::Close(frame) => {
                    if frame.is_some_and(|frame| u16::from(frame.code) == AUTHENTICATION_FAILED) {
                        bail!("The OBS WebSocket password is incorrect.");
                    }
                    bail!("OBS closed the connection.");
                }
                _ => {}
            }
        }
    }

    /// Sends one request; `Ok(None)` means OBS answered with a failed status.
    async fn request(&mut self, request_type: &str, data: Value) -> Result<Option<Value>> {
        let id = self.next_id.to_string();
        self.next_id += 1;
        self.send(json!({
            "op": 6,
            "d": { "requestType": request_type, "requestId": id, "requestData": data },
        }))
        .await?;
        loop {
            let response = self.receive_op(7).await?;
            if response["requestId"].as_str() != Some(id.as_str()) {
                continue;
            }
            return Ok(response["requestStatus"]["result"]
                .as_bool()
                .unwrap_or(false)
                .then(|| response["responseData"].clone()));
        }
    }

    async fn require(&mut self, request_type: &str, data: Value) -> Result<Value> {
        self.request(request_type, data)
            .await?
            .with_context(|| format!("OBS refused {request_type}."))
    }

    async fn close(mut self) {
        let _ = self
            .socket
            .close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: CloseCode::Normal,
                reason: "".into(),
            }))
            .await;
    }
}

pub async fn list_scenes(port: u16, password: Option<&str>) -> Result<ObsScenes> {
    let mut client = ObsClient::connect(port, password).await?;
    let data = client.require("GetSceneList", json!({})).await;
    client.close().await;
    let data = data?;
    // OBS lists scenes bottom-up; show them in the order of the Scenes dock.
    let mut scenes: Vec<String> = data["scenes"]
        .as_array()
        .map(|scenes| {
            scenes
                .iter()
                .filter_map(|scene| scene["sceneName"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    scenes.reverse();
    Ok(ObsScenes {
        scenes,
        current: data["currentProgramSceneName"].as_str().map(str::to_owned),
    })
}

/// True while OBS is streaming (used by the automatic live preset).
pub async fn stream_active(port: u16, password: Option<&str>) -> Result<bool> {
    let mut client = ObsClient::connect(port, password).await?;
    let data = client.require("GetStreamStatus", json!({})).await;
    client.close().await;
    Ok(data?["outputActive"].as_bool().unwrap_or(false))
}

/// Creates or updates each Relay source and makes sure it is in `scene`.
pub async fn install_sources(
    port: u16,
    password: Option<&str>,
    scene: &str,
    sources: &[SourceSpec],
) -> Result<Vec<InstalledSource>> {
    let mut client = ObsClient::connect(port, password).await?;
    let result = install_with(&mut client, scene, sources).await;
    client.close().await;
    result
}

async fn install_with(
    client: &mut ObsClient,
    scene: &str,
    sources: &[SourceSpec],
) -> Result<Vec<InstalledSource>> {
    let mut installed = Vec::new();
    for source in sources {
        let settings = browser_settings(&source.url);
        let exists = client
            .request("GetInputSettings", json!({ "inputName": source.name }))
            .await?
            .is_some();
        if !exists {
            client
                .require(
                    "CreateInput",
                    json!({
                        "sceneName": scene,
                        "inputName": source.name,
                        "inputKind": "browser_source",
                        "inputSettings": settings,
                        "sceneItemEnabled": true,
                    }),
                )
                .await?;
            installed.push(InstalledSource {
                name: source.name.to_owned(),
                action: "created",
            });
            continue;
        }
        client
            .require(
                "SetInputSettings",
                json!({ "inputName": source.name, "inputSettings": settings, "overlay": true }),
            )
            .await?;
        let in_scene = client
            .request(
                "GetSceneItemId",
                json!({ "sceneName": scene, "sourceName": source.name }),
            )
            .await?
            .is_some();
        let action = if in_scene {
            "updated"
        } else {
            client
                .require(
                    "CreateSceneItem",
                    json!({ "sceneName": scene, "sourceName": source.name, "sceneItemEnabled": true }),
                )
                .await?;
            "added"
        };
        installed.push(InstalledSource {
            name: source.name.to_owned(),
            action,
        });
    }
    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authentication_matches_the_obs_websocket_reference() {
        // Reference values from the obs-websocket 5 protocol documentation.
        assert_eq!(
            authentication(
                "supersecretpassword",
                "lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=",
                "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY="
            ),
            "1Ct943GAT+6YQUUX47Ia/ncufilbe6+oD6lY+5kaCu4="
        );
    }

    #[test]
    fn relay_sources_use_local_urls_and_optional_reactions() {
        let sources = relay_sources(4590, false);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].url, "http://localhost:4590/obs/visual");
        assert_eq!(sources[1].url, "http://127.0.0.1:4590/obs/audio");
        let with_reactions = relay_sources(4590, true);
        assert_eq!(with_reactions[2].name, "Relay Reactions");
        assert_eq!(with_reactions[2].url, "http://127.0.0.1:4590/reactions");
        for source in with_reactions {
            assert!(!source.url.contains("secret"));
        }
    }

    #[test]
    fn browser_sources_are_full_canvas_with_audio_in_the_mixer() {
        let settings = browser_settings("http://127.0.0.1:4590/obs/audio");
        assert_eq!(settings["width"], 1920);
        assert_eq!(settings["height"], 1080);
        assert_eq!(settings["reroute_audio"], true);
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    const PASSWORD: &str = "relay-test-password";
    const SALT: &str = "salt-value";
    const CHALLENGE: &str = "challenge-value";

    /// Minimal obs-websocket 5 server: password check, scene list, inputs.
    async fn fake_obs(existing_inputs: Vec<&'static str>) -> (u16, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = requests.clone();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let hello = json!({ "op": 0, "d": { "rpcVersion": 1,
                "authentication": { "salt": SALT, "challenge": CHALLENGE } } });
            socket.send(Message::text(hello.to_string())).await.unwrap();
            while let Some(Ok(message)) = socket.next().await {
                let Message::Text(text) = message else {
                    continue;
                };
                let value: Value = serde_json::from_str(text.as_str()).unwrap();
                match value["op"].as_u64() {
                    Some(1) => {
                        if value["d"]["authentication"]
                            != json!(authentication(PASSWORD, SALT, CHALLENGE))
                        {
                            let _ = socket
                                .close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                    code: CloseCode::from(AUTHENTICATION_FAILED),
                                    reason: "".into(),
                                }))
                                .await;
                            return;
                        }
                        socket
                            .send(Message::text(
                                json!({ "op": 2, "d": { "negotiatedRpcVersion": 1 } }).to_string(),
                            ))
                            .await
                            .unwrap();
                    }
                    Some(6) => {
                        let request = &value["d"];
                        let kind = request["requestType"].as_str().unwrap().to_owned();
                        let input = request["requestData"]["inputName"]
                            .as_str()
                            .or_else(|| request["requestData"]["sourceName"].as_str())
                            .unwrap_or_default()
                            .to_owned();
                        log.lock().unwrap().push(format!("{kind}:{input}"));
                        let (result, data) = match kind.as_str() {
                            "GetSceneList" => (
                                true,
                                json!({
                                    "currentProgramSceneName": "Live",
                                    "scenes": [{ "sceneName": "Break" }, { "sceneName": "Live" }],
                                }),
                            ),
                            "GetInputSettings" => {
                                (existing_inputs.contains(&input.as_str()), json!({}))
                            }
                            "GetSceneItemId" => (false, json!({})),
                            "GetStreamStatus" => (true, json!({ "outputActive": true })),
                            _ => (true, json!({})),
                        };
                        let response = json!({ "op": 7, "d": {
                            "requestType": kind, "requestId": request["requestId"],
                            "requestStatus": { "result": result, "code": if result { 100 } else { 600 } },
                            "responseData": data,
                        } });
                        socket
                            .send(Message::text(response.to_string()))
                            .await
                            .unwrap();
                    }
                    _ => {}
                }
            }
        });
        (port, requests)
    }

    #[tokio::test]
    async fn lists_scenes_in_dock_order_after_authenticating() {
        let (port, _) = fake_obs(vec![]).await;
        let scenes = list_scenes(port, Some(PASSWORD)).await.unwrap();
        assert_eq!(scenes.scenes, vec!["Live".to_owned(), "Break".to_owned()]);
        assert_eq!(scenes.current.as_deref(), Some("Live"));
    }

    #[tokio::test]
    async fn reports_whether_obs_is_streaming() {
        let (port, _) = fake_obs(vec![]).await;
        assert!(stream_active(port, Some(PASSWORD)).await.unwrap());
        let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed_port = closed.local_addr().unwrap().port();
        drop(closed);
        // OBS closed: an error, so the live mode keeps its current state.
        assert!(stream_active(closed_port, Some(PASSWORD)).await.is_err());
    }

    #[tokio::test]
    async fn a_wrong_password_is_reported_clearly() {
        let (port, _) = fake_obs(vec![]).await;
        let error = list_scenes(port, Some("wrong"))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("password is incorrect"), "{error}");
    }

    #[tokio::test]
    async fn a_missing_password_explains_where_to_find_it() {
        let (port, _) = fake_obs(vec![]).await;
        let error = list_scenes(port, None).await.unwrap_err().to_string();
        assert!(error.contains("WebSocket password"), "{error}");
    }

    #[tokio::test]
    async fn creates_new_sources_and_updates_existing_ones() {
        let (port, requests) = fake_obs(vec!["Relay Audio"]).await;
        let installed = install_sources(port, Some(PASSWORD), "Live", &relay_sources(4590, false))
            .await
            .unwrap();
        assert_eq!(installed[0].name, "Relay Visual");
        assert_eq!(installed[0].action, "created");
        assert_eq!(installed[1].name, "Relay Audio");
        assert_eq!(installed[1].action, "added");
        let requests = requests.lock().unwrap().clone();
        assert!(requests.contains(&"CreateInput:Relay Visual".to_owned()));
        assert!(requests.contains(&"SetInputSettings:Relay Audio".to_owned()));
        assert!(requests.contains(&"CreateSceneItem:Relay Audio".to_owned()));
    }

    #[tokio::test]
    async fn an_unreachable_obs_explains_how_to_enable_the_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let error = list_scenes(port, None).await.unwrap_err().to_string();
        assert!(error.contains("WebSocket Server Settings"), "{error}");
    }
}
