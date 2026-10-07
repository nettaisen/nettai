--!strict
-- DeltaRay, the navi of EXE4's DeltaRay chip (EXE4's actor object #0x3D, 0x080C7238), and its controller, which brings
-- him: EXE6's and EXE5's (@exelib/deltaray/navi; the same code) with EXE4's constants (ProtoMan's sprite 08-07 in the
-- chip's palette, 0x0800B950: two palettes a variant; his sounds; his flash, effect row 3; his mark, row 0x60; his
-- slash, row 0x5E; his sword, attachment row 3, in its animation 7; his finish's animation 15; the follow effect's look
-- 4 for his burst, objects/follow_effect) and its differences: his spawner (0x080C76CE) keeps neither his user, nor its
-- facing, nor the element, his height his side; his search starts at column 2, or 5 for side 1 (0x080C73D6), his first
-- candidate behind the enemy navi (EXE6's); his strike is ProtoMan's slash (0x080C71AA), all at once, his sword away as
-- it ends; no white flashes.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/deltaray/navi")
local follow_effect = require("../../objects/follow_effect")
local navi_chips = require("../../lib/navi_chips")

-- 0x080CD6E0, as his strike asks (attachment row 3, his sword's animation, animating while dimmed).
local function attach(owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?): Object?
    local anim = if options then options.anim or 0 else 0
    local while_dimmed = options ~= nil and options.while_dimmed == true
    return attachment.attach(owner, look, which, anim, while_dimmed)
end

local deltaray = common.make {
    id = "deltaray/navi",
    sprite = asset.sprite("protoman"),
    sounds = {
        appear = asset.sound("appear"),
        arrive = asset.sound("log-in"),
        mark = asset.sound("mark"),
        swing = asset.sound("sword-swing"),
        burst = asset.sound("delta-ray-burst"),
    },
    -- Effect rows 3, 0x60 and 0x5E.
    flash = new.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 },
    mark = new.effect { sprite = asset.sprite("hit-sparks"), anim = 11, palette = 1 },
    slash = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
    attach = attach,
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
    sword_anim = 7,
    self_type = collision.slash,
    target = collision.hits_navis,
    finish_anim = 15,
    burst = function(owner: Object, pos: Vec3, flip: number): Object?
        return follow_effect.spawn(owner, pos, follow_effect.looks.burst, flip)
    end,
    flashes = false,
    candidate_x = common.behind,
    tracks = false,
    summoned_alone = true,
    first_column = function(me: Object): number
        return if me.alliance == 0 then 2 else 5
    end,
    strike_at_once = true,
}

-- What the DeltaRay chip states for its controller: the controller's id (the original's effect object #0x42,
-- 0x080E6A18, for all six navis' chips: compat maps each to it) and his palette (0x0800B950 by the chip's level, 0:
-- ProtoMan's two palettes a variant).
export type ChipSpec = { id: string, palette: number }

-- The DeltaRay chip's controller: the user warps out, DeltaRay, the user warps back in.
local function controller(chip: ChipSpec): NaviChipController
    local summon = deltaray.summon { palette = chip.palette }
    -- 0x080C76CE: DeltaRay for the controller's user, with the damage plus the bonus.
    local function spawn(c: Object, user: Object, damage: number): Object?
        return summon(user, c, { panel_x = c.panel_x, panel_y = c.panel_y, element = c.element, damage = damage })
    end
    return navi_chips.controller { id = chip.id, name = "DeltaRay", spawn = spawn, steps = navi_chips.SHARED }
end

return { controller = controller }
