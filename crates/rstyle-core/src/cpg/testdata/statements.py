"""Chained comparisons and del statements."""


class Version:
    """A version that compares by its own method."""

    def __lt__(self, other: "Version") -> bool:
        return True


def between(lo: int, x: int, hi: int) -> bool:
    """Compares numbers, which runs no method."""
    return lo <= x < hi


def ordered(a: Version, b: Version, c: Version) -> bool:
    """Compares versions, which runs their own method, and stops at the first that fails."""
    return a < b < c


def dropped(d: dict[str, int], k: str) -> None:
    """Removes an item."""
    del d[k]


def forgotten(holder: Version, n: int) -> None:
    """Removes an attribute and a name."""
    del holder.name, n


def unpacked(a: int, b: int) -> None:
    """Removes names given as a tuple, which nothing models."""
    del (a, b)
