"""Nodes the Python frontend registers as its own, each inside a scan of a list it reads only."""

import asyncio


def labels(xs: list[int]) -> None:
    """Formats each item, which can run the value's own __format__."""
    for x in xs:
        label = f"item {x:>4}"


def constant(xs: list[int]) -> None:
    """Joins an f-string that formats nothing."""
    for x in xs:
        label = f"item"


def rows(xs: list[int]) -> None:
    """Builds a dict display per item, which hashes its keys."""
    for x in xs:
        row = {"value": x, "square": x * x}


def tails(xs: list[int], ys: list[int]) -> None:
    """Slices a list per item."""
    for x in xs:
        tail = ys[x:]


def spread(xs: list[int], ys: list[int]) -> None:
    """Unpacks a list into a tuple per item, which iterates it."""
    for x in xs:
        both = (*ys, x)


async def waits(future: asyncio.Future[int]) -> int:
    """Awaits a future, which runs its __await__."""
    return await future


def imports() -> None:
    """Imports a module, which can run its code, and binds its name."""
    import json


def named(options: dict[str, bool]) -> dict[str, bool]:
    """Passes one keyword by name and unpacks the rest."""
    return dict(strict=True, **options)
