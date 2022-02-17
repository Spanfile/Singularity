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
use std::sync::RwLock;

const DEFAULT_SINGULARITY_CONFIG_NAME: &str = "Default configuration";

#[derive(Debug)]
pub struct ConfigManager {
    // TODO: keep track of the current config's name? otoh it means figuring out when its name changes so maybe don't?
    active_config: RwLock<SingularityConfig>,
}

impl ConfigManager {
    pub fn load(conn: &mut DbConn) -> EvhResult<Self> {
        use crate::database::schema::evh_settings;

        let active_config_id: Option<DbId> = evh_settings::table
            .filter(evh_settings::setting_type.eq(EvhSettingType::ActiveSingularityConfig))
            .first::<models::EvhSetting>(conn) // read the setting from the DB
            .optional() // turn it into an Option so it is allowed to not exist
            .map_err(EvhError::from)? // convert the diesel error into an EvhError
            .map(|setting| {
                // try to parse the option's inner value into the proper type DbId
                setting
                    .value
                    .parse()
                    .map_err(|_| EvhError::InvalidSetting(EvhSettingType::ActiveSingularityConfig, setting.value))
            })
            // convert the Option<Result<...>> into Result<Option<...>> in order to handle the possible parsing error
            .transpose()?;

        debug!("Stored active config ID: {:?}", active_config_id);

        let cfg = match active_config_id {
            // if the config setting exists, attempt to use that config...
            Some(id) => {
                info!("Using active configuration with ID {}", id);
                SingularityConfig::load(id, conn)?.1
            }
            // otherwise try to load the first config that the database happens to return...
            None => {
                warn!("Active configuration ID not set, loading first found");

                let (_, cfg) = SingularityConfig::load_first(conn).or_else(|e| {
                    if let EvhError::Database(diesel::result::Error::NotFound) = e {
                        // if all else fails create a new config with the default name
                        warn!("No existing Singularity config found, falling back to creating a new one");

                        let cfg = SingularityConfig::new(conn, DEFAULT_SINGULARITY_CONFIG_NAME)?;
                        cfg.add_builtin_output(conn)?;

                        Ok((DEFAULT_SINGULARITY_CONFIG_NAME.to_string(), cfg))
                    } else {
                        Err(e)
                    }
                })?;

                // since the active configuration wasn't set and it now effectively is this one we just loaded, store it
                store_active_config(cfg.id(), conn)?;

                cfg
            }
        };

        Ok(Self {
            active_config: RwLock::new(cfg),
        })
    }

    pub fn get_active_config(&self) -> SingularityConfig {
        *self
            .active_config
            .read()
            .expect("configmanager active_config rwlock is poisoned")
    }

    pub fn set_active_config(&self, conn: &mut DbConn, cfg: SingularityConfig) -> EvhResult<()> {
        *self
            .active_config
            .write()
            .expect("configmanager active_config rwlock is poisoned") = cfg;

        store_active_config(cfg.id(), conn)
    }
}

fn store_active_config(id: DbId, conn: &mut DbConn) -> EvhResult<()> {
    use crate::database::schema::evh_settings;

    let value = id.to_string();
    let new_setting = diesel::insert_into(evh_settings::table)
        .values(models::NewEvhSetting {
            setting_type: EvhSettingType::ActiveSingularityConfig,
            // TODO: the column type in the database is TEXT, so this conversion is necessary to keep diesel happy.
            // while technically SQLite wouldn't bat an eye if I gave it an integer anyways, diesel does care to ensure
            // the data type is correct. maybe there's a way to avoid the conversion with a custom type that accepts
            // both strings and integers?
            value: &value,
        })
        // make the statement into an UPSERT
        .on_conflict(evh_settings::setting_type)
        .do_update()
        .set(evh_settings::value.eq(&value))
        .get_result::<models::EvhSetting>(conn)?;

    debug!("Stored new active config setting: {:?}", new_setting);
    Ok(())
}
