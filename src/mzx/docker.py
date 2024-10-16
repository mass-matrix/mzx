import loguru
import subprocess


def check_running() -> bool:
    """
    Check if Docker is running by executing the `docker info` command.

    Returns:
        bool: True if Docker is running (command succeeded), False otherwise.
    """
    try:
        subprocess.run(
            ["docker", "info"],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return True
    except subprocess.CalledProcessError as e:
        loguru.logger.exception(e)
        return False
