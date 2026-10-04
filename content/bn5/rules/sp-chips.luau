--!strict
-- BN5's SP navi chips' damage (0x0800E8DE, BN6's `sub_8010AE4` in the operation battle branch): the navis whose
-- deletion times a battle's setup carries, in its order (the save's 0x0203E560, 22 halfwords a side: an SP navi
-- chip's formula less one names its navi's slot), and the deletion-time steps (BCD hours, minutes, seconds,
-- hundredths; 0x0800E93C, BN6's) that pick the damage. Slot 0 is Roll's, whose SP chip counts holy panels instead
-- (0x0800E8B4); Django's SP chip counts turns (0x0800E98A).

return {
    slots = {
        "sp/roll",
        "sp/protoman",
        "sp/gyroman",
        "sp/serchman",
        "sp/napalmmn",
        "sp/magnetmn",
        "sp/meddy",
        "sp/colonel",
        "sp/shadoman",
        "sp/numbrman",
        "sp/tmhwkman",
        "sp/knightmn",
        "sp/toadman",
        "sp/shademan",
        "sp/blizman",
        "sp/cloudman",
        "sp/cosmoman",
        "sp/larkman",
        "sp/gridman",
    },
    deletion_times = {
        0x00001000,
        0x00001200,
        0x00001400,
        0x00001600,
        0x00001800,
        0x00002000,
        0x00002200,
        0x00002400,
        0x00002600,
        0x00002800,
    },
}
