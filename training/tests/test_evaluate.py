"""evaluate's duplicate deals (§9): envs keyed in pairs the other way round, and the deals' outcomes."""

from __future__ import annotations

import pytest

from nettai_ai.evaluate import Player, bot_pairs, duplicate_keys, duplicate_summary, round_robin_pairs


def test_each_env_shares_its_deal_with_one_of_its_pair_the_other_way_round():
    a, b, c = Player("a"), Player("b"), Player("c")
    for pairs in (round_robin_pairs([a, b, c], 12), bot_pairs(a, ["masher", "shooter"], 8)):
        keys = duplicate_keys(pairs)
        for k in set(keys):
            e, f = [i for i, x in enumerate(keys) if x == k]
            assert pairs[e][0] is pairs[f][1] and pairs[e][1] is pairs[f][0]
    with pytest.raises(ValueError, match="other way round"):
        duplicate_keys([(a, b), (a, b)])


def test_a_deal_is_won_both_times_lost_both_times_or_split():
    def rec(key, n, players, result, truncated=False):
        return {"game": 0, "deal": (key, n), "players": players, "result": result, "truncated": truncated}

    ab, ba = ("a", "b"), ("b", "a")
    records = [
        rec(0, 0, ab, (1, -1)), rec(0, 0, ba, (-1, 1)),  # a won with both seats' builds
        rec(0, 1, ab, (1, -1)), rec(0, 1, ba, (1, -1)),  # seat 0's build won both times
        rec(0, 2, ab, (-1, 1)), rec(0, 2, ba, (1, -1)),  # b both times
        rec(0, 3, ab, (0, 0)), rec(0, 3, ba, (1, -1)),  # a draw
        rec(0, 4, ab, (1, -1), True), rec(0, 4, ba, (1, -1)),  # cut
    ]
    # Another pairing's deals of the same keys (another call of run_matches) are their own.
    records += [rec(0, 0, ("a", "c"), (1, -1)), rec(0, 0, ("c", "a"), (1, -1))]
    assert duplicate_summary(records, ["exe6"]) == {
        ("exe6", "a", "b"): {"deals": 5, "first": 1, "second": 1, "split": 1, "other": 2},
        ("exe6", "a", "c"): {"deals": 1, "first": 0, "second": 0, "split": 1, "other": 0},
    }
