import mock
import subprocess
from src.mzx import docker  # Adjust to your module's path


@mock.patch("src.mzx.docker.subprocess.run", return_value=None)
def test_docker_running(mock_run):
    """Test the case where Docker is running successfully."""
    result = docker.check_running()

    mock_run.assert_called_once_with(
        ["docker", "info"], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    assert result is True


@mock.patch("src.mzx.docker.subprocess.run")
def test_docker_not_running(mock_run):
    """Test the case where Docker is not running and command fails."""
    # Mock subprocess.run to raise CalledProcessError to simulate failure
    mock_run.side_effect = subprocess.CalledProcessError(1, ["docker", "info"])

    result = docker.check_running()

    mock_run.assert_called_once_with(
        ["docker", "info"], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    assert result is False
