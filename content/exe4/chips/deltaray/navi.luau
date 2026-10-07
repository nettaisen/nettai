--!strict
-- DeltaRay, the navi of EXE4's DeltaRay chip (EXE4's actor object #0x3D, 0x080C7238), whom the shared navi chip
-- controller (objects/navi_chip_controller, its entry 0xC) brings: EXE6's and EXE5's (@exelib/deltaray/navi; the same
-- code) with EXE4's constants (ProtoMan's sprite 08-07 in the chip's palette, 0x0800B950: two palettes a variant;
-- his sounds; his flash, effect row 3; his mark, row 0x60; his slash, row 0x5E; his sword, attachment row 3, in its
-- animation 7; his finish's animation 15; the follow effect's look 4 for his burst, ./follow_effect) and its
-- differences: his spawner (0x080C76CE) keeps neither his user, nor its facing, nor the element, his height his side;
-- his search starts at column 2, or 5 for side 1 (0x080C73D6), his first candidate behind the enemy navi (EXE6's);
-- his strike is ProtoMan's slash (0x080C71AA), all at once, his sword away as it ends; no white flashes.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/deltaray/navi")
local follow_effect = require("./follow_effect")
local navi_chips = require("../../lib/navi_chips")

-- His palettes a variant (0x0800B9B1's entry 7, ProtoMan's).
local PALETTES = 2

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

local summon = deltaray.summon { palette = navi_chips.palette(PALETTES, 0) }

-- 0x080C76CE: DeltaRay for the controller's user, with the damage plus the bonus. (The chip's level is 0.)
local function spawn(controller: Object, user: Object, damage: number): Object?
    return summon(
        user,
        controller,
        { panel_x = controller.panel_x, panel_y = controller.panel_y, element = controller.element, damage = damage }
    )
end

return { kind = deltaray.kind, spawn = spawn }
