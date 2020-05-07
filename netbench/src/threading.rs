use config::Config;

pub fn setup(config: &Config) {
    if config.p_id >= 0 {
        let core_ids = core_affinity::get_core_ids().unwrap();
        core_affinity::set_for_current(core_ids[(config.p_id as usize) % core_ids.len()]);
    }
}