--!strict
-- Otenko's statue (the original's attack object #0xAD, `sub_80DB0E4`;
-- docs/engine/dimming-chip-effects.md §9) and the chip's dimming
-- controller (effect object #0x5F, `sub_80E7624`): a field obstacle with
-- 100 HP that stands for 1800 ticks in front of the user and, every 5
-- ticks, adds 1 to the Atk+ bonus of the chip at the user's hand's cursor
-- if it deals damage, up to 50 for each hand entry. The behavior is EXE5's
-- too (@exelib/otenko/otenko), made here with EXE6's look.

local absorbed_obstacle = require("../../objects/absorbed_obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/otenko")
local effects = require("../../lib/effects")

-- Its sprite (`sub_80DB108`'s, in `sub_80DB2C6`'s pool), with a shadow,
-- animation 0: the Japanese ROMs' 0C-49, Otenko (animation 0 the puff he
-- appears in, 1 Otenko). The US ROMs' 0C-49 is a placeholder, and their
-- statue loads 0C-00, the swords' sheet, instead; the content shows the
-- Japanese games' Otenko on every console (docs/engine/jp-differences.md
-- §5).
local SPRITE = asset.sprite("otenko")

return common.make {
    statue_id = "otenko/statue",
    controller_id = "otenko/controller",
    sprite = SPRITE,
    -- Its identity: a field object, which looks as Otenko when something
    -- takes its place (`byte_8021220`'s row for its NameID: DustMan's
    -- junk). That is the Japanese ROMs' row; the US ROMs' is the table's
    -- none, so DustMan throws no junk for it there
    -- (docs/engine/jp-differences.md §8, row 16). The content follows the
    -- Japanese games on every console.
    identity = define.identity {
        id = "otenko",
        class = "field_object",
        version = 2,
        ai_index = 1,
        look = { sprite = SPRITE, anim = 1, palette = 0, shadow = true },
    },
    lifetime = 0x708,
    -- One of its side's two field objects of class 0 (a third evicts the
    -- oldest).
    class = 0,
    sounds = { appear = asset.sound("appear"), ["break"] = asset.sound("panel-poison") },
    -- Another body of its side.
    is = collision.own_body,
    hits = collision.hit_by_other_side,
    -- How it looks absorbed (the original's obstacle kind 0xE, a row of
    -- `byte_80E98C0`): its own sprite, the Japanese ROMs' row. (The US
    -- ROMs' row is the AirSpin top's sprite, 0C-41.)
    absorbed = absorbed_obstacle.look { sprite = SPRITE },
    effects = { puff = effects.puff, bubbles = effects.bubbles },
    special_bonus = false,
}
