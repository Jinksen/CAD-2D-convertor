"""Entry point of the frozen geometry service shipped inside the installer and portable ZIP."""

import ctypes
import os
import threading

import uvicorn

from cad2maxwell_backend.api import app
from cad2maxwell_backend.config import Settings

SYNCHRONIZE = 0x00100000
INFINITE = 0xFFFFFFFF


def exit_with_parent(pid: int) -> None:
    """Stop when the desktop window process ends, even if it crashed, so its port is freed."""
    kernel32 = ctypes.windll.kernel32  # type: ignore[attr-defined]
    handle = kernel32.OpenProcess(SYNCHRONIZE, False, pid)
    if not handle:
        os._exit(1)

    def wait() -> None:
        kernel32.WaitForSingleObject(handle, INFINITE)
        os._exit(0)

    threading.Thread(target=wait, daemon=True).start()


def main() -> None:
    parent = os.environ.get("CAD2MAXWELL_PARENT_PID")
    if parent:
        exit_with_parent(int(parent))
    settings = Settings()
    # log_config=None: the windowed build has no console, uvicorn's default formatter would fail.
    uvicorn.run(app, host=settings.host, port=settings.port, log_config=None)


if __name__ == "__main__":
    main()
