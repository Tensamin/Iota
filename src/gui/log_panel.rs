use crate::ACTIVE_TASKS;
use crate::APP_STATE;
use crate::SHUTDOWN;
use crate::gui::tui::UNIQUE;
use crate::langu::language_manager::format;
use crate::langu::language_manager::from_key;

use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};

use std::{thread, time::Duration};
use sysinfo::{RefreshKind, System};

pub fn log_cv(cv: &CommunicationValue) {
    if cv.is_type(CommunicationType::identification_response) {
        let args = [cv.get_data(DataTypes::accepted).unwrap().as_str().unwrap()];
        log_message(format(&"identification_response", &args));
    } else {
        log_message_trans(format!("{:?}", &cv.comm_type));
    }
    tokio::spawn(async move {
        *UNIQUE.write().await = true;
    });
}
pub fn log_message_trans(key: impl Into<String>) {
    APP_STATE.lock().unwrap().push_log(from_key(&key.into()));
    tokio::spawn(async move {
        *UNIQUE.write().await = true;
    });
}
pub fn log_message(msg: impl Into<String>) {
    APP_STATE.lock().unwrap().push_log(msg.into());
    tokio::spawn(async move {
        *UNIQUE.write().await = true;
    });
}

pub fn setup() {
    // Start a background thread to sample metrics
    tokio::spawn(async move {
        {
            ACTIVE_TASKS.lock().unwrap().push("metrics".to_string());
        }
        let mut sys = System::new_with_specifics(RefreshKind::new());
        let mut last_total_received = 0u64;
        let mut last_total_transmitted = 0u64;
        let mut counter = 0.0;
        loop {
            if *SHUTDOWN.read().await {
                break;
            }
            sys.refresh_all();

            let mut tcpu = 0;
            for cpu in sys.cpus() {
                tcpu += cpu.cpu_usage() as i64;
                tcpu /= 2;
            }
            let ram = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;

            let total_received = 0u64;
            let total_transmitted = 0u64;

            let delta_received = if last_total_received == 0 {
                0
            } else {
                total_received.saturating_sub(last_total_received)
            };
            let delta_transmitted = if last_total_transmitted == 0 {
                0
            } else {
                total_transmitted.saturating_sub(last_total_transmitted)
            };
            last_total_received = total_received;
            last_total_transmitted = total_transmitted;

            let net_down = delta_received as f64;
            let net_up = delta_transmitted as f64;

            {
                let mut st = APP_STATE.lock().unwrap();
                st.push_cpu((counter, tcpu as f64));
                st.push_ram((counter, ram));
                st.push_net_down((counter, net_down));
                st.push_net_up((counter, net_up));

                st.sys_info = format!("NetDown: {}  NetUp: {}", delta_received, delta_transmitted);
            }

            counter += 1.0;
            *UNIQUE.write().await = true;
            thread::sleep(Duration::from_millis(1000));
        }
        {
            ACTIVE_TASKS
                .lock()
                .unwrap()
                .retain(|t| !t.eq(&"metrics".to_string()));
        }
    });
}
