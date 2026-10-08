--!strict
-- EXE5's ElemRage (action 0x56, 0x080EFCD0): EXE5's own. The navi raises its claw (the attachment table's row 0x34,
-- 0c-33, its animation 1) and sends a flame (attack object #0x98, 0x080D99D8) along its row from the panel ahead, at
-- its attach point 20; each flame, when its animation first ends, sends the next one ahead until the record's count
-- (its first parameter, 3) is spent, and stands its time (the second, 30 ticks). On lava, sea, ice, grass or magnet
-- the navi takes that panel's element (0x080EFDE4: fire, aqua, aqua, wood, elec) and palette for the flames, which
-- then also spread to the rows beside and paralyze, and its own panel turns back to normal (the panel changer's row
-- 11). Its roar sounds every 16 ticks; 31 ticks in, 6 more, and it is back to idle.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local flame_common = require("@exelib/elemrage/flame")
local panel_changer = require("../../objects/panel_changer")
local sparks = require("../../lib/sparks")

local elemrage = {}

-- ---- The flame ------------------------------------------------------------------------------------------------------

export type FlameState = ElemFlameState

-- The flame (attack object #0x98, 0x080D99D8): the shared flame (@exelib/elemrage/flame) with EXE5's sprite and
-- collision types; a spreading flame sends the two beside its next one. (EXE5's flames leave no panel type: ElemRage
-- and the fire arm set none in Param4's high nibble.)
local FLAME = flame_common.make_kind {
    id = "elemrage/flame",
    sprite = asset.sprite("rage-flame"),
    spreads = true,
    attack = collision.attack,
    attack_480 = collision.attack_480,
    hits = collision.hits_navis,
    spark = sparks.of_element,
}
elemrage.flame = FLAME.kind
local spawn_flame = FLAME.spawn

-- ---- The action -----------------------------------------------------------------------------------------------------

export type State = {
    -- Ticks into the action (AIAttackVars+0x10) and the roar's count (+0x12).
    timer: number,
    roar: number,
    -- The flames' palette (+0x30) and whether the navi took a panel's element (+0x0F bit 2).
    palette: number,
    raged: boolean,
}

-- The panel types that give an element (0x080EFDE4): the element, the flames' palette.
local RAGES: { [string]: { element: number, palette: number } } = {
    lava = { element = 1, palette = 0 },
    sea = { element = 2, palette = 1 },
    ice = { element = 2, palette = 1 },
    grass = { element = 4, palette = 2 },
    magnet = { element = 3, palette = 3 },
}
local NONE = { element = 0, palette = 4 }
local CLAW = attachment.look { sprite = asset.sprite("rage-claw"), palette = 0, attach_point = 0x16 }
-- (The flame and the claw MegaMan's B↓FireAm fires too: navis/megaman/weapons/fire_arm.)
elemrage.spawn_flame = spawn_flame
elemrage.claw = CLAW
local ROAR, RAGE_ROAR = asset.sound("roar"), asset.sound("rage-roar")
-- The user's panel back to normal (the panel changer's row 11: its own panel, 30 ticks, flickering).
local RESTORE = panel_changer.change {
    panel = "normal",
    panels = "own",
    flicker = true,
    ticks = 30,
    sides = { [0] = { require = 0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } },
    tick_sound = panel_changer.TICK_SOUND,
    change_sound = panel_changer.CHANGE_SOUND,
}
local ANIM, ARM_ANIM, ATTACH_POINT = 10, 1, 20
local LAST_TICK, RECOVER_TICKS = 30, 5

-- ElemRage's action: its flames' count and time (the record's parameters; its flags, the third, 0).
function elemrage.action(count: number, ticks: number): Action
    -- 0x080EFCEC: the rage by the panel; the flame out; the claw raised; the roar every 16 ticks; on after the 31st.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            local p = field.panel(me.panel_x, me.panel_y)
            local rage = (p and RAGES[p.kind]) or NONE
            me.attack_element = rage.element
            s.palette = rage.palette
            s.raged = rage.element ~= 0
            if s.raged then
                panel_changer.spawn(me, me.panel_x, me.panel_y, RESTORE, nil)
            end
            -- (The damage word takes the Atk+ bonus, AIAttackVars+8.)
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            -- (`sub_8018842` reads the attach point unflipped: the flame turns it its own way.)
            local fx, dz = me:attach_point(ATTACH_POINT)
            local dx = fx * me.facing
            spawn_flame(me, me.panel_x + me.facing, me.panel_y, me.attack_element, int.shl(dz, 16), {
                count = count,
                spread = s.raged,
                ticks = ticks,
                offset = int.i8(dx),
                paralyzes = s.raged,
                attack_480 = false,
                while_dimmed = false,
                palette = s.palette,
                hit = false,
                sent = false,
            }, damage)
            me.anim = ANIM
            me:open_counter_window()
            attachment.attach(me, CLAW, "overlay", { anim = ARM_ANIM })
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        s.roar = bit32.band(s.roar + 1, 15)
        if s.roar == 1 then
            battle.play_sound(if s.raged then RAGE_ROAR else ROAR)
        end
        s.timer += 1
        if s.timer >= LAST_TICK then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080EFDFC: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    return new.action {
        state = { timer = "u16", roar = "u16", palette = "u8", raged = "bool" },
        -- 0x080EFCD0.
        update = function(me: Object, s: State)
            if me.step == 0 then
                fire(me, s)
            elseif me.step == 4 then
                recover(me, s)
            else
                error(string.format("ElemRage phase %#x reads past 0x080EFCE4", me.step))
            end
        end,
    }
end

return elemrage
