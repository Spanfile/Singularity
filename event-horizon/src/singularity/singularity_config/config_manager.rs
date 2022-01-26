// TODO: terrible name

use super::SingularityConfig;
use crate::{
    database::{
        models::{self, EvhSettingType},
        DbConn, DbId,
    },
    error::{EvhError, EvhResult},
};
use diesel::prelude::*;
use log::*;
use std::sync::Arc;

// this is actually determined by the database
const DEFAULT_SINGULARITY_CONFIG_ID: DbId = 1;
const DEFAULT_SINGULARITY_CONFIG_NAME: &str = "Default configuration";

#[derive(Debug, Clone)]
pub struct ConfigManager {
    // TODO: not a fan of how these two values are separated from each other
    active_config_name: Arc<String>,
    active_config: Arc<SingularityConfig>,
}

impl ConfigManager {
    pub fn load(conn: &mut DbConn) -> EvhResult<Self> {
        use crate::database::schema::evh_settings;

        let type_id = EvhSettingType::ActiveSingularityConfig as DbId;
        let active_config_id: Option<DbId> = evh_settings::table
            .filter(evh_settings::setting_type.eq(type_id))
            .first::<models::EvhSetting>(conn) // read the setting from the DB
            .optional() // turn it into an Option so it is allowed to not exist
            .map_err(EvhError::from)? // convert the diesel error into an EvhError
            .map(|setting| {
                // try to parse the option's inner value into the proper type DbId
                setting
                    .value
                    .parse()
                    .map_err(|_| EvhError::InvalidSetting(type_id, setting.value))
            })
            // convert the Option<Result<...>> into Result<Option<...>> in order to handle the possible parsing error
            .transpose()?;

        debug!("Stored active config ID: {:?}", active_config_id);

        let (name, cfg) = match active_config_id {
            // if the config setting exists, attempt to use that config...
            Some(id) => {
                info!("Using active configuration with ID {}", id);
                SingularityConfig::load(id, conn)?
            }
            // otherwise try to load the config with the default ID if it exists...
            None => {
                warn!(
                    "Active configuration ID not set, using default ID {}",
                    DEFAULT_SINGULARITY_CONFIG_ID
                );

                let (name, cfg) = SingularityConfig::load(DEFAULT_SINGULARITY_CONFIG_ID, conn).or_else(|e| {
                    if let EvhError::Database(diesel::result::Error::NotFound) = e {
                        // if all else fails create a new config with the default name
                        warn!("No existing Singularity config found, falling back to creating a new one");

                        Ok((
                            DEFAULT_SINGULARITY_CONFIG_NAME.to_string(),
                            SingularityConfig::new(conn, DEFAULT_SINGULARITY_CONFIG_NAME)?,
                        ))
                    } else {
                        Err(e)
                    }
                })?;

                // since the active configuration wasn't set and it now effectively is this one we just loaded, store it
                let new_setting = diesel::insert_into(evh_settings::table)
                    .values(models::NewEvhSetting {
                        setting_type: type_id,
                        // TODO: this is kinda stupid, to allocate a new string just to borrow it for diesel to store in
                        // the database as an integer again
                        value: &cfg.id().to_string(),
                    })
                    .get_result::<models::EvhSetting>(conn)?;

                debug!("Stored new active config setting: {:?}", new_setting);

                (name, cfg)
            }
        };

        Ok(Self {
            active_config_name: Arc::new(name),
            active_config: Arc::new(cfg),
        })
    }

    pub fn get_active_config(&self) -> Arc<SingularityConfig> {
        Arc::clone(&self.active_config)
    }

    pub fn get_active_config_name(&self) -> &str {
        self.active_config_name.as_str()
    }
}
