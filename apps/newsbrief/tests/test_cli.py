from newsbrief import cli, config


def test_init_writes_loadable_templates(tmp_path):
    assert cli.main(["init", str(tmp_path)]) == 0
    cfg = config.load(tmp_path / "config.toml")
    assert cfg.triage.model == "claude-opus-5" and cfg.triage.effort == "low"
    assert cfg.db_path == tmp_path / "newsbrief.db"
    assert cfg.feeds and "About me" in cfg.profile()
