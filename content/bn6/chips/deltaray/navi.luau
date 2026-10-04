--!strict
-- DeltaRay, the navi of the DeltaRay chip (the original's actor object
-- #0x3D, `sub_80C2A78`): the shared DeltaRay (@exelib/deltaray/navi), BN6's
-- kind. He fades in on his user's panel, flashing, then looks for the first
-- enemy navi ahead (column by column, rows 1 to 3) and a free panel
-- diagonally behind and above it; there he strikes with his sword (a
-- column of three in front of him, with a slash effect). Each time his user
-- presses A while dimmed during a strike, he goes on to the next corner of
-- the delta (up to three strikes), as long as the panel is free; then he
-- returns home, flashes the screen with a burst, and leaves. Without an
-- enemy navi ahead, he only marks the air. See docs/engine/chip-table.md
-- (navi chip subtype 28).

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/deltaray/navi")
local follow_effect = require("../../objects/follow-effect")
local parts = require("../../lib/swords/parts")

-- What a chip summons him with: his palette (the chip's first parameter
-- picks it).
export type Summon = { palette: number }

-- His bursts' look (the follow effect's look 4).
local BURST = follow_effect.look { sprite = asset.sprite("follow-effect"), anim = 0, palette = 0, behavior = "flash" }

return common.make {
    id = "deltaray/navi",
    sprite = asset.sprite("protoman"),
    sounds = {
        appear = asset.sound("appear"),
        arrive = asset.sound("log-in"),
        mark = asset.sound("mark"),
        swing = parts.sounds.swing,
        burst = asset.sound("delta-ray-2"),
    },
    -- The flash above him as he arrives (effect 0x03), and the mark on his
    -- target (effect 0x28); his slash drawn over the column (effect 0x27).
    flash = define.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 },
    mark = define.effect { sprite = asset.sprite("hit-sparks"), anim = 11, palette = 1 },
    slash = parts.effects.wide,
    -- His sword: the swords' blade (the original's attachment 3), its
    -- animation 0x0B, stepping while dimmed.
    attach = attachment.attach,
    sword = parts.blades.sword,
    sword_anim = 0x0B,
    self_type = collision.slash,
    target = collision.hits_navis,
    finish_anim = 0xF,
    burst = function(owner: Object, pos: Vec3, flip: number): Object?
        return follow_effect.spawn(owner, pos, BURST, flip)
    end,
    flashes = true,
    candidate_x = common.behind,
    tracks = false,
}
