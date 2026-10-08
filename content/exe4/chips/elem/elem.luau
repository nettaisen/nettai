--!strict
-- EXE4's Elem chips' action (0x60, 0x080EE2AE: EXE4's own), its variants 0 to 4 (ElemFlar, ElemIce, ElemLeaf, ElemSand,
-- ElemDark), each chip's own (`elem.action`) with its variant's rows. The navi raises its claw (attachment row 0x2C,
-- 0c-45, its animation three times the navi's number, in the related slot) and sends a flame (attack object 9:
-- @exelib/elemrage/flame, EXE5's ElemRage flame) along its row from the panel ahead, 4 pixels back and 26 up
-- (0x080EE3D0 by the navi's number: MegaMan's). On the chip's panel (lava, ice, grass, sand, poison: 0x080EE3EF) its
-- flames run five panels and paralyze (each variant's flag 4, 0x080EE3F4; ElemDark's also leave poison, the high
-- nibble's 4), and its own panel turns back to normal (the panel changer's row 11). The flames are the chip's palette
-- (0x080EE3EA). Its roar sounds every 16 ticks; 6 ticks in its claw's animation steps on; 30 ticks in, then 6 more
-- with the claw stepped again, and it is back to idle.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local flame_common = require("@exelib/elemrage/flame")
local panel_changer = require("../../objects/panel_changer")
local sparks = require("../../lib/sparks")

local elem = {}

-- The flame (attack object 9, 0x080CE2D8): EXE4's sprite (0c-0e); ElemDark's leave poison; a spreading flame sends none
-- beside.
local FLAME = flame_common.make_kind {
    id = "elem/flame",
    sprite = asset.sprite("rage-flame"),
    leaves = { "poison" },
    spreads = false,
    attack = collision.attack,
    attack_480 = collision.attack_480,
    hits = collision.hits_navis,
    spark = sparks.of_element,
}

-- The navi's number (NaviStats +0x23: MegaMan's 0) and its row of 0x080EE3D0: the flame's offset ahead and height.
local NAVI_NUMBER = 0
local OFFSET, HEIGHT = -4, 26

local CLAW = attachment.look { sprite = asset.sprite("rage-claw"), palette = 0 }
local ROAR, RAGE_ROAR = asset.sound("roar"), asset.sound("rage-roar")
-- The user's panel back to normal (the panel changer's row 11: its own panel, 30 ticks, flickering).
local RESTORE = panel_changer.change {
    panel = "normal",
    panels = "own",
    flicker = true,
    ticks = 30,
    sides = panel_changer.OWN_AREA,
    tick_sound = panel_changer.TICK_SOUND,
    change_sound = panel_changer.CHANGE_SOUND,
}
local ANIM = 13
local CLAW_STEP, LAST_TICK, RECOVER_TICKS = 6, 30, 5
-- A raging flame's count (0x080EE2F6); it paralyzes (0x080EE3F4's flag 4, each variant's).
local RAGING_COUNT = 5

-- A chip's Elem action: its key, the panel that rages it (0x080EE3EF), its flames' palette (0x080EE3EA) and the panel
-- type its raging flames leave (0x080EE3F4's high nibble: ElemDark's poison), its flames' count and time (the record's
-- parameters +0x10 to +0x12), and whether its flames' self type is row 0x25 (`attack_480`: its flags, +0x13, 8:
-- ElemDark's; else 0).
type ElemSpec = {
    id: string,
    rages_on: PanelType,
    palette: number,
    raging_leaves: PanelType?,
    count: number,
    ticks: number,
    attack_480: boolean?,
}

type State = {
    -- Ticks into the action (AIAttackVars+0x10), the roar's count (+0x12), and whether it raged (its flames' Param4's
    -- bit 2).
    timer: number,
    roar: number,
    raged: boolean,
}

function elem.action(spec: ElemSpec): Action
    -- 0x080EE2CC: on its first tick the rage by the panel, the flame out, the claw raised; the roar every 16 ticks;
    -- the claw's step 6 ticks in; on after the 30th.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            local count = spec.count
            local leaves: PanelType? = nil
            local p = field.panel(me.panel_x, me.panel_y)
            local raged = p ~= nil and p.kind == spec.rages_on
            if raged then
                leaves = spec.raging_leaves
                count = RAGING_COUNT
                panel_changer.spawn(me, me.panel_x, me.panel_y, RESTORE, nil)
            end
            s.raged = raged
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            FLAME.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, HEIGHT * 0x10000, {
                count = bit32.band(count, 0x7F),
                spread = bit32.btest(count, 0x80),
                ticks = spec.ticks,
                offset = OFFSET,
                paralyzes = raged,
                attack_480 = spec.attack_480 == true,
                while_dimmed = false,
                leaves = leaves,
                palette = spec.palette,
                hit = false,
                sent = false,
            }, damage)
            me.anim = ANIM
            me:open_counter_window()
            attachment.attach(me, CLAW, "related", 3 * NAVI_NUMBER)
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        s.roar = bit32.band(s.roar + 1, 15)
        if s.roar == 1 then
            battle.play_sound(if s.raged then RAGE_ROAR else ROAR)
        end
        if s.timer == CLAW_STEP then
            local claw = me.related1
            if claw then
                claw.anim += 1
            end
        end
        s.timer += 1
        if s.timer >= LAST_TICK then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080EE3FC: the claw's step again, 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            local claw = me.related1
            if claw then
                claw.anim += 1
            end
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = { timer = "u16", roar = "u16", raged = "bool" },
        -- 0x080EE2AE.
        update = function(me: Object, s: State)
            if me.step == 0 then
                fire(me, s)
            elseif me.step == 4 then
                recover(me, s)
            else
                error(string.format("an Elem chip's phase %#x reads past 0x080EE2C0", me.step))
            end
        end,
    }
end

return elem
