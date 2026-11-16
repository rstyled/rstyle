"""Loops over ranges and tuples, which count their passes without a list to measure."""


def counted(n: int) -> int:
    """Counts up to n."""
    total = 0
    for i in range(n):
        total += i
    return total


def offset(n: int) -> int:
    """Counts from one up to n."""
    total = 0
    for i in range(1, n):
        total += i
    return total


def stepped(n: int) -> int:
    """Counts up to n by twos."""
    total = 0
    for i in range(0, n, 2):
        total += i
    return total


def between(a: int, b: int) -> int:
    """Counts from a start that can be negative, which bounds nothing."""
    total = 0
    for i in range(a, b):
        total += i
    return total


def backward(n: int) -> int:
    """Counts down, which a stop doesn't bound."""
    total = 0
    for i in range(n, 0, -1):
        total += i
    return total


def nested(n: int, m: int) -> int:
    """Counts up to m for each count up to n."""
    total = 0
    for i in range(n):
        for j in range(m):
            total += j
    return total


def pairs(a: int, b: int) -> int:
    """Scans a tuple display."""
    total = 0
    for x in (a, b, a + b):
        total += x
    return total


def numbered(xs: list[int]) -> int:
    """Scans a list through enumerate."""
    total = 0
    for i, x in enumerate(xs):
        total += x
    return total


def paired(xs: list[int], ys: list[int]) -> int:
    """Scans two lists together, no further than the first."""
    total = 0
    for x, y in zip(xs, ys):
        total += x * y
    return total


def backwards(xs: list[int]) -> int:
    """Scans a list from its end."""
    total = 0
    for x in reversed(xs):
        total += x
    return total


def ordered(xs: list[int]) -> int:
    """Scans a sorted copy, which nothing counts."""
    total = 0
    for x in sorted(xs):
        total += x
    return total


def indexed(n: int) -> int:
    """Numbers a range."""
    total = 0
    for i, j in enumerate(range(n)):
        total += j
    return total


def alone(xs: list[int]) -> int:
    """Zips one list, which zip has no role for."""
    total = 0
    for (x,) in zip(xs):
        total += x
    return total


def untyped(n):
    """Counts up to a value of no known integer type."""
    total = 0
    for i in range(n):
        total += 1
    return total
