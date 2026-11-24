fn archive_after_shutdown(result: Result<()>) -> Result<()> {
    fahare_horizon::finish_shutdown(result.is_ok());
    result
}
