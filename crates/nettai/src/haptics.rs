//! A light tap felt as a finger presses a button of the on-screen pad (docs/app.md §11), on a phone: iOS's impact
//! feedback (light), Android's keyboard tap. Each system leaves it out where its player turned such feedback off (iOS's
//! System Haptics, Android's Touch feedback), and neither is waited on: the frame goes on as the system plays it. The
//! desktop and a browser have nothing to do.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

/// A light tap, now.
pub fn tap() {
    #[cfg(target_os = "android")]
    android::tap();
    #[cfg(target_os = "ios")]
    ios::tap();
}
