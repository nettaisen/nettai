--!strict
-- Hub Style's shade (the body overlays' row 5, 0c-58), as an identity wears it: what MegaMan's base form puts on in Hub
-- Style (NaviStats +0x4C: ./base), and the DS's Double Attacks' MegaMan (lib/double_attack). In front. Its depth table
-- is 16 bytes long; past it his animations read on into the tables' pointers and the sprite table (none 0: in front)
-- and the code after them (its first byte 0: behind, his animation 52).

local HUB_SHADE: BodyPart = {
    sprite = asset.sprite("hub-shade"),
    anims = 53,
    behind = { { 52, 52 } },
}

return new.identity { class = "navi", parts = { body = HUB_SHADE } }
