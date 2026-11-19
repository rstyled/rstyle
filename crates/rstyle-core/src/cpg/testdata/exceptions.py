"""Raising and handling exceptions around scans of lists the functions only read."""


class Empty(Exception):
    """Raised for an empty list."""


def checked(xs: list[int]) -> None:
    """Raises inside a scan, which leaves it early."""
    for x in xs:
        if x < 0:
            raise Empty


def asserted(xs: list[int]) -> None:
    """Asserts before a scan."""
    assert xs, "empty"
    for x in xs:
        pass


def handled(xs: list[int], ys: list[int]) -> None:
    """Handles what each item's division raises by scanning another list."""
    for x in xs:
        try:
            share = 100 // x
        except ZeroDivisionError:
            for y in ys:
                pass


def finished(xs: list[int], ys: list[int]) -> int:
    """Returns from inside a scan, which still runs the finally block's scan."""
    try:
        for x in xs:
            if x == 0:
                return 0
    finally:
        for y in ys:
            pass
    return 1


def interrupted(n: int) -> None:
    """A limit set partway through a body that raised is unknown to the loop after it."""
    limit = 0
    i = 0
    try:
        limit = n
        share = 100 // n
        limit = 0
    except ZeroDivisionError:
        pass
    while i < limit:
        i += 1


def managed(lock: object, xs: list[int]) -> None:
    """Scans under a context manager, entered and exited once."""
    with lock:
        for x in xs:
            pass


def bound(lock: object, xs: list[int]) -> None:
    """Binds what a context manager's enter returns to a name it writes."""
    with lock as held:
        for x in xs:
            pass


def caught(xs: list[int], ys: list[int]) -> None:
    """Raises inside a try, which its handler catches."""
    for x in xs:
        try:
            if x < 0:
                raise Empty
        except Empty:
            for y in ys:
                pass


def skipped(xs: list[int], ys: list[int]) -> None:
    """Continues from inside a try, through its finally block."""
    for x in xs:
        try:
            if x < 0:
                continue
        finally:
            for y in ys:
                pass


def rebound(lock: object, xs: list[int]) -> None:
    """Binds what a context manager's enter returns to a name bound before."""
    held = None
    with lock as held:
        for x in xs:
            pass


class Holder:
    """Keeps what a context manager's enter returns."""

    held: object = None

    def kept(self, lock: object, xs: list[int]) -> None:
        """Binds it to an attribute, which writes part of a value."""
        with lock as self.held:
            for x in xs:
                pass
