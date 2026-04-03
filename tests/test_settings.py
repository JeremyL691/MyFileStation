import json

from myfilestation.settings import AppSettings, SettingsService


def test_settings_service_round_trip(tmp_path):
    settings_path = tmp_path / "settings.json"
    service = SettingsService(path=str(settings_path))

    settings = AppSettings(
        dock_side="left",
        remove_after_drag_out=False,
        autostart=True,
        cleanup_temp_on_exit=False,
    )
    service.save(settings)

    loaded = service.load()

    assert loaded == settings
    assert service.last_warning is None


def test_settings_service_recovers_from_broken_json(tmp_path):
    settings_path = tmp_path / "settings.json"
    settings_path.write_text("{not-json", encoding="utf-8")

    service = SettingsService(path=str(settings_path))
    loaded = service.load()

    assert isinstance(loaded, AppSettings)
    assert loaded.cleanup_temp_on_exit is True
    assert service.last_warning is not None


def test_settings_service_reads_missing_cleanup_field_as_default(tmp_path):
    settings_path = tmp_path / "settings.json"
    settings_path.write_text(
        json.dumps(
            {
                "dock_side": "left",
                "remove_after_drag_out": False,
                "autostart": True,
            }
        ),
        encoding="utf-8",
    )

    service = SettingsService(path=str(settings_path))
    loaded = service.load()

    assert loaded.cleanup_temp_on_exit is True
