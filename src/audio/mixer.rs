//! Group routing for managed playback. Raw playback deliberately bypasses it.
use super::{SoundId, SoundManager};
use crate::settings::{finite, GameSettings};
use macroquad::audio::{play_sound, set_sound_volume, PlaySoundParams};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioGroup {
    Sfx,
    Music,
    Voice,
    Ui,
    Ambience,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Playback {
    pub group: AudioGroup,
    pub gain: f32,
}

pub fn group_volume(settings: &GameSettings, group: AudioGroup, focused: bool) -> f32 {
    if settings.mute_when_unfocused && !focused {
        return 0.0;
    }
    let group = match group {
        AudioGroup::Sfx => settings.sfx_volume,
        AudioGroup::Music => settings.music_volume,
        AudioGroup::Voice => settings.voice_volume,
        AudioGroup::Ui => settings.ui_volume,
        AudioGroup::Ambience => settings.ambience_volume,
    };
    finite(settings.master_volume, 0.0, 1.0, 1.0) * finite(group, 0.0, 1.0, 1.0)
}

impl<T: SoundId> SoundManager<T> {
    /// Call after Apply and whenever focus changes. Updates ongoing sounds.
    /// `focused` comes from the host's focus/visibility event integration.
    pub fn apply_settings(&mut self, settings: &GameSettings, focused: bool) {
        self.settings = settings.clone();
        self.settings.sanitize();
        self.focused = focused;
        self.sfx_volume = group_volume(&self.settings, AudioGroup::Sfx, focused);
        self.music_volume = group_volume(&self.settings, AudioGroup::Music, focused);
        for (id, playback) in self.managed.borrow().iter() {
            if let Some(sound) = self.sounds.get(id) {
                set_sound_volume(sound, self.managed_volume(*playback));
            }
        }
    }

    fn managed_volume(&self, playback: Playback) -> f32 {
        let volume = match playback.group {
            AudioGroup::Sfx => self.sfx_volume,
            AudioGroup::Music => self.music_volume,
            group => group_volume(&self.settings, group, self.focused),
        };
        finite(volume * playback.gain, 0.0, 1.0, 0.0)
    }

    /// Route one sound ID to a group, including looping music and ambience.
    /// Macroquad controls volume by sound asset, so simultaneous instances of
    /// the same ID share its latest group/gain. Use distinct IDs for distinct buses.
    pub fn play_group(&self, id: T, group: AudioGroup, gain: f32, looped: bool) {
        if !self.visible {
            return;
        }
        if let Some(sound) = self.sounds.get(&id) {
            let playback = Playback {
                group,
                gain: finite(gain, 0.0, 1.0, 1.0),
            };
            self.managed.borrow_mut().insert(id, playback);
            play_sound(
                sound,
                PlaySoundParams {
                    looped,
                    volume: self.managed_volume(playback),
                },
            );
        }
    }
}

#[cfg(test)]
mod tests;
