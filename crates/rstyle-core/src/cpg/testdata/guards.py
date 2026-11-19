"""While loops whose condition calls something."""


def ready(i: int) -> bool:
    """Says whether to go on."""
    return i < 10


def polled(n: int) -> int:
    """Calls ready before every pass and once more, through continue too."""
    i = 0
    while ready(i):
        i += 1
        if i == n:
            continue
    return i


def counted(n: int) -> int:
    """Counts up to n, with no call in its condition."""
    i = 0
    while i < n:
        i += 1
    return i
