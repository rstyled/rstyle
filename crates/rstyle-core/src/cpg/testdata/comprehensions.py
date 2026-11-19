"""Comprehensions over sources the loop can count."""


def squares(xs: list[int]) -> list[int]:
    """Builds a list of the positive items' squares."""
    return [x * x for x in xs if x > 0]


def table(n: int, m: int) -> dict[int, int]:
    """Builds a dict from two nested ranges."""
    return {i: j for i in range(n) for j in range(m)}


def unique(xs: list[int]) -> set[int]:
    """Builds a set, which hashes each item."""
    return {x for x in xs}


def summed(xs: list[int]) -> int:
    """Consumes a generator in the call it's an argument of."""
    return sum(x for x in xs)


def spread(xs: list[int]) -> int:
    """Unpacks a generator into the call it's an argument of."""
    return max(0, *(x for x in xs))


def stored(xs: list[int]) -> int:
    """Keeps a generator to consume later, which can run against values that changed."""
    pending = (x for x in xs)
    return sum(pending)
