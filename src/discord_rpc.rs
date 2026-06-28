use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use log::{info, warn};

// Create a Discord Application at https://discord.com/developers/applications
// and upload mudular.png as a Rich Presence asset with key "mudular".
const APP_ID: &str = "1520929378404536462";

pub struct DiscordPresence {
  client: DiscordIpcClient
}

impl Drop for DiscordPresence {
  fn drop(&mut self) {
    let _ = self.client.close();
    info!("Discord RPC closed");
  }
}

pub fn start(details: &str) -> Option<DiscordPresence> {
  let mut client = DiscordIpcClient::new(APP_ID);
  if let Err(e) = client.connect() {
    warn!("Discord RPC connect failed (is Discord running?): {e}");
    None?
  }
  let activity = activity::Activity::new()
    .details(details)
    .assets(
      activity::Assets::new()
        .large_image("mudular")
        .large_text("MUDular")
    );
  if let Err(e) = client.set_activity(activity) {
    warn!("Discord RPC set_activity failed: {e}");
    None?
  }
  info!("Discord RPC started: {details}");
  Some(DiscordPresence { client })
}
