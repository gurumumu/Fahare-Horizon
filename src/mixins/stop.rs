fn request_clean_stop(state: &super::ServerState) {
    if fahare_horizon::take_stop_request() {
        if let Ok(mut lists) = state.lists.lock() {
            lists.stop = true;
        }
    }
}
