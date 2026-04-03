from pathlib import Path

from myfilestation import utils


def test_get_autostart_command_for_dev_uses_pythonw(monkeypatch, tmp_path):
    python_exe = tmp_path / "python.exe"
    python_exe.write_text("", encoding="utf-8")
    pythonw_exe = tmp_path / "pythonw.exe"
    pythonw_exe.write_text("", encoding="utf-8")

    monkeypatch.setattr(utils.sys, "executable", str(python_exe))
    monkeypatch.setattr(utils.sys, "frozen", False, raising=False)

    command = utils.get_autostart_command()

    assert str(pythonw_exe) in command
    assert "run_myfilestation.py" in command


def test_get_autostart_command_for_frozen_build(monkeypatch, tmp_path):
    app_exe = tmp_path / "MyFileStation.exe"
    app_exe.write_text("", encoding="utf-8")

    monkeypatch.setattr(utils.sys, "executable", str(app_exe))
    monkeypatch.setattr(utils.sys, "frozen", True, raising=False)

    command = utils.get_autostart_command()

    assert command == f'"{app_exe}"'


def test_create_temp_text_file_and_delete(tmp_path, monkeypatch):
    monkeypatch.setenv("TEMP", str(tmp_path))

    path = Path(utils.create_temp_text_file("hello"))
    assert path.exists()

    utils.delete_file_if_exists(str(path))
    assert not path.exists()


def test_open_with_default_app_raises_for_missing_file(tmp_path):
    missing = tmp_path / "missing.txt"

    try:
        utils.open_with_default_app(str(missing))
    except FileNotFoundError as exc:
        assert str(missing) in str(exc)
    else:
        raise AssertionError("Expected FileNotFoundError for missing file")
