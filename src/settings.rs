//! Extension settings, edited on the extension's card in the admin area.
use serde::{Deserialize, Serialize};
use shared::{
    State,
    extensions::settings::{
        ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt,
        SettingsSerializer,
    },
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Eggs whose servers get the mod installer; empty means every server that looks like a
    /// Vintage Story server (its egg, startup command or files).
    pub eggs: Vec<uuid::Uuid>,
    /// Largest mod archive read in full, in MiB, to find its `modinfo.json` when Wings can't
    /// open the archive itself (disabled mods).
    pub max_archive_mb: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            eggs: Vec::new(),
            max_archive_mb: 64,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if !(1..=1024).contains(&self.max_archive_mb) {
            errors.push("max_archive_mb must be between 1 and 1024".into());
        }
        if self.eggs.len() > 1000 {
            errors.push("eggs must list at most 1000 eggs".into());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// `Some(true)` when the admin listed the egg, `Some(false)` when they listed others, `None`
    /// when detection decides.
    pub fn egg_listed(&self, egg: uuid::Uuid) -> Option<bool> {
        (!self.eggs.is_empty()).then(|| self.eggs.contains(&egg))
    }
}

#[async_trait::async_trait]
impl SettingsSerializeExt for Settings {
    async fn serialize(
        &self,
        serializer: SettingsSerializer,
    ) -> Result<SettingsSerializer, anyhow::Error> {
        Ok(serializer
            .write_serde_setting("eggs", &self.eggs)?
            .write_serde_setting("max_archive_mb", &self.max_archive_mb)?)
    }
}

pub struct SettingsDataDeserializer;

#[async_trait::async_trait]
impl SettingsDeserializeExt for SettingsDataDeserializer {
    async fn deserialize_boxed(
        &self,
        deserializer: SettingsDeserializer<'_>,
    ) -> Result<ExtensionSettings, anyhow::Error> {
        let defaults = Settings::default();
        Ok(Box::new(Settings {
            eggs: deserializer
                .read_serde_setting("eggs")
                .unwrap_or(defaults.eggs),
            max_archive_mb: deserializer
                .read_serde_setting("max_archive_mb")
                .unwrap_or(defaults.max_archive_mb),
        }))
    }
}

pub async fn load(state: &State) -> Result<Settings, anyhow::Error> {
    let settings = state.settings.get().await?;
    Ok(settings
        .get_extension_settings::<Settings>(crate::PACKAGE)
        .cloned()
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_limits_are_checked() {
        assert!(Settings::default().validate().is_ok());
        let wrong = Settings {
            max_archive_mb: 0,
            ..Settings::default()
        };
        assert_eq!(wrong.validate().unwrap_err().len(), 1);
    }

    #[test]
    fn an_empty_egg_list_leaves_it_to_detection() {
        let egg = uuid::Uuid::from_u128(7);
        assert_eq!(Settings::default().egg_listed(egg), None);
        let only_other = Settings {
            eggs: vec![uuid::Uuid::from_u128(8)],
            ..Settings::default()
        };
        assert_eq!(only_other.egg_listed(egg), Some(false));
        assert_eq!(only_other.egg_listed(uuid::Uuid::from_u128(8)), Some(true));
    }
}
