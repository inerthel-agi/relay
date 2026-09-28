/* Shared connection plumbing for Relay browser outputs (OBS pages and widgets). */
const RelayConnection = {
  /** Secret from the page URL, or the meta tag that short OBS URLs carry. */
  secret(parameters) {
    return parameters.get("secret")
      || document.querySelector('meta[name="relay-secret"]')?.content
      || "";
  },

  /**
   * Relay moved to another port. OBS Browser Sources never retry a failed page
   * load, so the new server is probed before navigating to it.
   */
  moveToPort(port, probeUrl, isUnloading = () => false) {
    const nextUrl = new URL(window.location.href);
    nextUrl.port = String(port);
    const probe = new WebSocket(probeUrl(`${window.location.hostname}:${port}`));
    let ready = false;
    const probeWatchdog = window.setTimeout(() => {
      if (!ready) probe.close();
    }, 5000);
    probe.addEventListener("open", () => {
      ready = true;
      window.clearTimeout(probeWatchdog);
      probe.close();
      window.location.replace(nextUrl.href);
    });
    probe.addEventListener("close", () => {
      window.clearTimeout(probeWatchdog);
      if (!ready && !isUnloading()) {
        window.setTimeout(() => RelayConnection.moveToPort(port, probeUrl, isUnloading), 1000);
      }
    });
  },

  /**
   * A page kept open across a Relay restart (often an update) reconnects with
   * its old code. The returned check reloads it when the Relay session changes.
   */
  reloadOnRestart() {
    let session;
    return (next) => {
      if (typeof next !== "string" || !next) return false;
      session ??= next;
      if (next === session) return false;
      window.location.reload();
      return true;
    };
  },
};
