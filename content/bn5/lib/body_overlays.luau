--!strict
-- BN5's body overlays (actor object 0x56, 0x080C35E8, BN6's `sub_80C4348`: the same code, but it always wears its
-- wearer's palette and starts at animation 0): a second sprite layered on a navi, by row (Param1): its sprite
-- (0x080C35DC) and its depth table (0x080C35C4: a byte by the wearer's animation, 0 drawn a pixel behind it). An
-- identity wears one as its `parts` (`parts = { body = body_overlays.gyroman }`). Row 5 (12-58, the shade of
-- MegaMan's base form with NaviStats +0x4C, 0x0800EE1C) waits for that state, which bn5-compat refuses.

local body_overlays = {}

-- Row 0: GyroMan's propeller (the GyroMan chips' navi and GyroSoul's image, 0x0800EE4E): behind him in his
-- animations 0 to 5, 7, 16 and 17.
body_overlays.gyroman = {
    sprite = asset.sprite("bn5:gyro-propeller"),
    anims = 22,
    behind = { { 0, 5 }, { 7, 7 }, { 16, 17 } },
}

-- Row 1: KnightMan's ball and chain (the KnightMan chips' navi and KnightSoul's image, 0x0800EE7A): in front.
body_overlays.knightman = { sprite = asset.sprite("bn5:knightman-ball"), anims = 20 }

-- Row 2: MegaMan's propeller in GyroSoul (0x0800F06C): behind him in every animation.
body_overlays.gyrosoul = {
    sprite = asset.sprite("bn5:gyrosoul-propeller"),
    anims = 34,
    behind = { { 0, 33 } },
}

-- Row 3: Colonel's cape (the Colonel chips' navi and ColonelSoul's image, 0x0800EE5E): in front.
body_overlays.colonel = { sprite = asset.sprite("bn5:colonel-cape"), anims = 41 }

-- Row 4: NapalmMan's cannon (the NapalmMan chips' navi and NapalmSoul's image, 0x0800EE8A): behind him in his
-- animations 20 to 22.
body_overlays.napalmman = {
    sprite = asset.sprite("bn5:napalm-cannon"),
    anims = 23,
    behind = { { 20, 22 } },
}

return body_overlays
