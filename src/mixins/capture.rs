fn capture_shutdown(game: &mut Game<'_>, dedicated: bool, crashed: bool, run_error: Option<&str>) {
    let world = if dedicated && !crashed && run_error.is_none() && game.world_adopted && game.session_lock.is_some() {
        Some(game.world_root.clone())
    } else {
        None
    };
    fahare_horizon::capture_shutdown(world);
}
