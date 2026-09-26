use crate::reactions::TriggerResult;

pub fn reply(language: &str, result: anyhow::Result<TriggerResult>) -> String {
    let (index, position) = match result {
        Ok(result) if result.position == 0 => (0, None),
        Ok(result) => (7, Some(result.position)),
        Err(error) => {
            let text = error.to_string().to_ascii_lowercase();
            if text.contains("disabled") {
                (1, None)
            } else if (text.contains("queue") && text.contains("full"))
                || text.contains("requests are temporarily full")
            {
                (8, None)
            } else if text.contains("already playing") {
                (2, None)
            } else if text.contains("wait before") {
                (3, None)
            } else if text.contains("channel or your roles") {
                (4, None)
            } else if text.contains("no reaction output") {
                (6, None)
            } else {
                (5, None)
            }
        }
    };
    let messages = match language {
        "fr" => [
            "Réaction lancée.",
            "Les réactions sont désactivées.",
            "Une réaction est déjà en cours.",
            "Patiente avant de déclencher une autre réaction.",
            "Ce salon ou tes rôles ne permettent pas de déclencher des réactions.",
            "Cette réaction est indisponible.",
            "Redémarre Relay pour rétablir le son Windows, ou connecte la source OBS des réactions.",
            "Réaction en file (position n°{position}).",
            "La file des réactions est pleine.",
        ],
        "es" => [
            "Reacción iniciada.",
            "Las reacciones están desactivadas.",
            "Ya se está reproduciendo una reacción.",
            "Espera antes de activar otra reacción.",
            "Este canal o tus roles no permiten activar reacciones.",
            "Esta reacción no está disponible.",
            "Reinicia Relay para restaurar el audio de Windows o conecta la fuente OBS de reacciones.",
            "Reacción en cola (posición n.º {position}).",
            "La cola de reacciones está llena.",
        ],
        "de" => [
            "Reaktion gestartet.",
            "Reaktionen sind deaktiviert.",
            "Eine Reaktion wird bereits abgespielt.",
            "Warte, bevor du eine weitere Reaktion auslöst.",
            "Dieser Kanal oder deine Rollen erlauben keine Reaktionen.",
            "Diese Reaktion ist nicht verfügbar.",
            "Starte Relay für die Windows-Audioausgabe neu oder verbinde die OBS-Reaktionsquelle.",
            "Reaktion eingereiht (Position #{position}).",
            "Die Reaktionswarteschlange ist voll.",
        ],
        "ru" => [
            "Реакция запущена.",
            "Реакции отключены.",
            "Реакция уже воспроизводится.",
            "Подождите перед запуском следующей реакции.",
            "Этот канал или ваши роли не разрешают запуск реакций.",
            "Эта реакция недоступна.",
            "Перезапустите Relay для восстановления звука Windows или подключите источник реакций OBS.",
            "Реакция в очереди (позиция №{position}).",
            "Очередь реакций заполнена.",
        ],
        "zh" => [
            "反应已启动。",
            "反应已禁用。",
            "已有反应正在播放。",
            "请稍后再触发反应。",
            "此频道或你的角色不允许触发反应。",
            "此反应不可用。",
            "重启 Relay 以恢复 Windows 音频，或连接 OBS 反应源。",
            "反应已排队（位置 #{position}）。",
            "反应队列已满。",
        ],
        "ko" => [
            "반응을 시작했습니다.",
            "반응이 비활성화되어 있습니다.",
            "이미 반응이 재생 중입니다.",
            "다음 반응을 실행하기 전에 기다려 주세요.",
            "이 채널 또는 현재 역할로는 반응을 실행할 수 없습니다.",
            "이 반응을 사용할 수 없습니다.",
            "Relay를 다시 시작하여 Windows 오디오를 복원하거나 OBS 반응 소스를 연결하세요.",
            "반응이 대기열에 추가되었습니다(#{position}번째).",
            "반응 대기열이 가득 찼습니다.",
        ],
        "ja" => [
            "リアクションを開始しました。",
            "リアクションは無効です。",
            "リアクションがすでに再生中です。",
            "次のリアクションを実行する前にお待ちください。",
            "このチャンネルまたはロールではリアクションを実行できません。",
            "このリアクションは利用できません。",
            "Relayを再起動してWindows音声を復元するか、OBSリアクションソースを接続してください。",
            "リアクションをキューに追加しました（{position}番目）。",
            "リアクションキューがいっぱいです。",
        ],
        "id" => [
            "Reaksi dimulai.",
            "Reaksi dinonaktifkan.",
            "Reaksi sedang diputar.",
            "Tunggu sebelum memicu reaksi berikutnya.",
            "Kanal atau peran Anda tidak diizinkan memicu reaksi.",
            "Reaksi ini tidak tersedia.",
            "Mulai ulang Relay untuk memulihkan audio Windows, atau hubungkan sumber reaksi OBS.",
            "Reaksi masuk antrean (posisi #{position}).",
            "Antrean reaksi penuh.",
        ],
        _ => [
            "Reaction started.",
            "Reactions are disabled.",
            "A reaction is already playing.",
            "Wait before triggering another reaction.",
            "This channel or your roles cannot trigger reactions.",
            "This reaction is unavailable.",
            "Restart Relay to restore Windows audio, or connect the OBS reaction source.",
            "Reaction queued (position #{position}).",
            "The reaction queue is full.",
        ],
    };
    match position {
        Some(position) => messages[index].replace("{position}", &position.to_string()),
        None => messages[index].into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_queue_admission_is_distinct_from_started_in_every_language() {
        for language in ["en", "fr", "es", "de", "ru", "zh", "ko", "ja", "id"] {
            let started = reply(language, Ok(TriggerResult { position: 0 }));
            let queued = reply(language, Ok(TriggerResult { position: 12 }));
            assert_ne!(started, queued);
            assert!(queued.contains("12"));
            assert!(!queued.contains("{position}"));
        }
    }

    #[test]
    fn full_queue_reports_refusal_instead_of_success() {
        assert_eq!(
            reply("fr", Err(anyhow::anyhow!("Reaction queue is full."))),
            "La file des réactions est pleine."
        );
        assert_eq!(
            reply("en", Ok(TriggerResult { position: 0 })),
            "Reaction started."
        );
    }
}
