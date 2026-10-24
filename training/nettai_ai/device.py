"""The device and the threads that share it (§8's loop): which device, each thread's own queue of device work,
copies that don't wait, and each thread's torch CPU threads."""

from __future__ import annotations

import contextlib

import torch


def pick_device(name: str) -> torch.device:
    """"auto": CUDA where there is one, else the CPU (the Mac's MPS was slower than its CPU at this size, §11)."""
    if name == "auto":
        return torch.device("cuda" if torch.cuda.is_available() else "cpu")
    return torch.device(name)


def pin_torch_threads(n: int) -> None:
    """The calling thread's torch CPU threads (0 leaves torch's default). torch keeps the count per thread, but a
    thread's first parallel op resets it to the process's last setting, so one is run before setting it again."""
    if n <= 0:
        return
    torch.set_num_threads(n)
    torch.zeros(1 << 17).tanh_()
    torch.set_num_threads(n)


def synchronize(device: torch.device) -> None:
    """Every lane's work so far is done (for setting up, before the threads start)."""
    if device.type == "cuda":
        torch.cuda.synchronize(device)


def upload(t: torch.Tensor, device: torch.device) -> torch.Tensor:
    """A CPU tensor on the device, without waiting: through pinned memory on CUDA (a copy from pageable memory
    would wait for the stream's queued work first)."""
    if device.type == "cuda":
        return t.pin_memory().to(device, non_blocking=True)
    return t.to(device)


class Lane:
    """One thread's queue of device work. On CUDA it is a stream of its own, so the collector's small kernels and
    its wait for each step's actions never sit behind the learner's queued update (the collector's stream has
    the higher priority: the pool waits on it). Elsewhere a thread's ops run in that thread, in order, and there
    is nothing to keep apart; marks are None."""

    def __init__(self, device: torch.device, *, default: bool = False, priority: int = 0):
        self.device = device
        self.stream = None
        if device.type == "cuda":
            self.stream = torch.cuda.default_stream(device) if default else torch.cuda.Stream(device, priority=priority)

    def enter(self) -> None:
        """Make it the calling thread's current stream (each thread has its own)."""
        if self.stream is not None:
            torch.cuda.set_stream(self.stream)

    def scope(self):
        """Its stream as the current one inside a with block (for allocating what this lane will use)."""
        return torch.cuda.stream(self.stream) if self.stream is not None else contextlib.nullcontext()

    def mark(self):
        """A point after this lane's work so far, for another thread to wait for."""
        if self.stream is None:
            return None
        ev = torch.cuda.Event()
        ev.record(self.stream)
        return ev

    def follow(self, mark) -> None:
        """This lane's next work starts after mark (the calling thread doesn't wait)."""
        if mark is not None:
            self.stream.wait_event(mark)

    @staticmethod
    def wait(mark) -> None:
        """The calling thread waits until the work before mark is done (host memory it read may be written)."""
        if mark is not None:
            mark.synchronize()

    def synchronize(self) -> None:
        if self.stream is not None:
            self.stream.synchronize()
