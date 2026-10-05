--!strict
-- Dark MegaMan (NameID 0x18D), whom a failed Chaos Unison brings onto the other side (rules/souls/chaos): a navi no
-- player controls (docs/design/exe5-map.md §15.9). His actor record (0x08014C94's row: version 0, actor type navi, AI
-- index 0x16, which EXE5's computer-navi AI answers for: rules/computer-navi); his enemy structs (0x081041C0: MegaMan's
-- battle sprite 00-00 with a shadow, palette scale 1; 0x081041C8: 500 HP, no element, flags 9, his body dealing 10);
-- his collision (0x08013C5A: for NameID 0x18D his own body's row 0x13 and the navi target's row 2, hit modifier 3);
-- what his flags give (superarmor, affected by ice) and his post-init hook (0x08104270: palette 1, untouchable, his
-- HP number gone). He wears MegaMan's sprite, so its attach points.

local attach_points = require("../megaman/attach_points")
local collision = require("../../rules/collision")

return define.identity {
    id = "dark-megaman",
    class = "navi",
    version = 0,
    actor_type = "navi",
    ai_index = 0x16,
    attach_points = attach_points[0],
    body = {
        sprite = asset.sprite("megaman"),
        shadow = true,
        palette_scale = 1,
        palette = 1,
        hp = 500,
        element = "null",
        body_damage = 10,
        collision = { body = collision.own_body, target = collision.navi_target, hit_modifier = 3 },
        flags = { "superarmor", "affected_by_ice", "untouchable" },
        hides_hp = true,
    },
}
