--!strict
-- EXE5's body overlays (actor object 0x56, 0x080C35E8, EXE6's `sub_80C4348`: the same code, but it always wears its
-- wearer's palette and starts at animation 0): a second sprite layered on a navi, by row (Param1): its sprite
-- (0x080C35DC) and its depth table (0x080C35C4: a byte by the wearer's animation, 0 drawn a pixel behind it). An
-- identity wears one as its `parts` (`parts = { body = body_overlays.gyroman }`).

local body_overlays = {}

-- Row 0: GyroMan's propeller (the GyroMan chips' navi and GyroSoul's image, 0x0800EE4E): behind him in his
-- animations 0 to 5, 7, 16 and 17.
body_overlays.gyroman = {
    sprite = asset.sprite("gyro-propeller"),
    anims = 22,
    behind = { { 0, 5 }, { 7, 7 }, { 16, 17 } },
}

-- Row 1: KnightMan's ball and chain (the KnightMan chips' navi and KnightSoul's image, 0x0800EE7A): in front.
body_overlays.knightman = { sprite = asset.sprite("knightman-ball"), anims = 20 }

-- Row 2: MegaMan's propeller in GyroSoul (0x0800F06C): behind him in every animation.
body_overlays.gyrosoul = {
    sprite = asset.sprite("gyrosoul-propeller"),
    anims = 34,
    behind = { { 0, 33 } },
}

-- Row 3: Colonel's cape (the Colonel chips' navi and ColonelSoul's image, 0x0800EE5E): in front.
body_overlays.colonel = { sprite = asset.sprite("colonel-cape"), anims = 41 }

-- Row 4: NapalmMan's cannon (the NapalmMan chips' navi and NapalmSoul's image, 0x0800EE8A): behind him in his
-- animations 20 to 22.
body_overlays.napalmman = {
    sprite = asset.sprite("napalm-cannon"),
    anims = 23,
    behind = { { 20, 22 } },
}

-- Row 5: Hub Style's shade (0c-58), which MegaMan's base form puts on in Hub Style (NaviStats +0x4C, 0x0800EE1C:
-- navis/megaman/forms/base): in front. Its depth table is 16 bytes long; past it his animations read on into the
-- tables' pointers and the sprite table (none 0: in front) and the code after them (its first byte 0: behind, his
-- animation 52).
body_overlays.hub = {
    sprite = asset.sprite("hub-shade"),
    anims = 53,
    behind = { { 52, 52 } },
}

return body_overlays
