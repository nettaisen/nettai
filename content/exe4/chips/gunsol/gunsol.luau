--!strict
-- EXE4's GunSol (action 0x58, 0x080EDE0E: EXE5's GunDelSol, action 0x2F, grew from it): GunSol1 to 3's and GunSolEX's,
-- each chip's own (`gunsol.action`: its gun, firing time, beams, region and the columns it lights). The navi
-- (animation 13, a counter window) holds out the gun (its attachment row, 0x1F and the variant: its animation three
-- times the navi's number) with its sound; 6 ticks later, A held or not, the gun's animation goes one on and the sun
-- beam lights up 80 pixels ahead (@exelib/gundels/beam, effect 0x48, watching the gun's slot; the attack keeps it,
-- AIAttackVars +0x1C), and then each tick, while the navi's player holds A (a navi in auto battle, NaviStats +0x26,
-- needn't; nor would the story's dark soul, +0x24 15) and the chip's firing time lasts, a drain hit (element 5, no
-- spark, hit modifier 0, a thrown piercing break hitting navis) of 2 damage (4 in the sun, NaviStats +0x29) on the
-- column two panels ahead, whose holes the light fills (the Hole chip's, type 11, turned normal), and GunSolEX's the
-- column past it too. Then the beam goes, and 11 ticks later (the gun's animation one more on as they start) the navi
-- puts the gun away and is back to idle. EXE5's but: no test of A as the beam lights, nor of the battle's end as it
-- fires; the gun in the navi's first related slot; the holes filled.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/gundels/beam")

local gunsol = {}

-- The sun beam (effect object #0x48, 0x080E7134: EXE5's, its hum 0x160).
local BEAM = common.make_kind("gunsol/beam", asset.sound("sun-beam"))

-- A chip's GunSol: its key; its gun (attachment rows 0x1F to 0x22: 0c-3b in palettes 0, 3, 6, 9); how long it fires
-- at most (0x080EDEFC); its beam in the shade and in the sun (0x080EDEEC's looks: 0x080E7130's sprite and a palette);
-- its hits' region (0x080EDFD4's, 0x080EDFD8's); the columns whose holes it fills, from the one it hits.
export type Spec = {
    id: string,
    gun: AttachmentLook,
    firing_ticks: number,
    beam: SunBeamLook,
    beam_in_sun: SunBeamLook,
    region: Region,
    columns: number,
}

type State = {
    -- Ticks left in the phase (AIAttackVars +0x10), and the beam (+0x1C).
    timer: number,
    beam: Object?,
}

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local ANIM = 13
local SOUND = asset.sound("gundels1")
local WIND_UP_TICKS, RECOVER_TICKS = 6, 10
-- The beam, 80 pixels ahead of the navi.
local BEAM_AHEAD = 0x50
-- The hits: element 5, two panels ahead, 2 damage (4 in the sun).
local ELEMENT, AHEAD = 5, 2
local DAMAGE, SUN_DAMAGE = 2, 4
-- What the light fills, and what it leaves.
local HOLE: PanelType, NORMAL: PanelType = "hole", "normal"

local WIND_UP, FIRING, RECOVER = 0x0, 0x4, 0x8

-- 0x080EDF00's test: a player who must hold A to keep firing has let go of it.
local function let_go(me: Object): boolean
    if me.actor_type ~= "player" then
        return false
    end
    return not me:held("a")
end

-- 0x080EDF68: the holes in column x, its three rows, turned normal.
local function fill(x: number, y: number)
    for row = y - 1, y + 1 do
        local p = field.panel(x, row)
        if p and p.kind == HOLE then
            field.set_type(x, row, NORMAL)
        end
    end
end

function gunsol.action(spec: Spec): Action
    -- 0x080EDE44: the gun out with its sound; 6 ticks later its animation one on, the beam lit, firing.
    local function wind_up(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me.anim, me.anim_loaded = ANIM, 0xFF
            me:open_counter_window()
            attachment.attach(me, spec.gun, "related", 3 * NAVI_NUMBER)
            battle.play_sound(SOUND)
            s.timer = WIND_UP_TICKS
            me.step_init = 4
            return
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t > 0 then
            return
        end
        s.timer = spec.firing_ticks
        local gun = me.related1
        if gun then
            gun.anim += 1
        end
        local look = if battle.navi(me.alliance).sun then spec.beam_in_sun else spec.beam
        s.beam = common.spawn(BEAM, me, look, Vec3.px(me.facing * BEAM_AHEAD, 0, 0), "related", false, true)
        me.step, me.step_init = FIRING, 0
    end

    -- 0x080EDF00: a hit this tick, the holes filled, until A is let go or the time runs out; then the beam goes.
    local function fire(me: Object, s: State)
        local stop = let_go(me)
        if not stop then
            local t = s.timer - 1
            s.timer = int.u16(t)
            stop = t < 0
        end
        if stop then
            local b = s.beam
            if b then
                common.stop(b)
            end
            me.step, me.step_init = RECOVER, 0
            return
        end
        local x, y = me.panel_x + AHEAD * me.facing, me.panel_y
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = ELEMENT,
            z = 0,
            region = spec.region,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.thrown_piercing_break,
            damage = if battle.navi(me.alliance).sun then SUN_DAMAGE else DAMAGE,
            hit_mod = 0,
        })
        for c = 0, spec.columns - 1 do
            fill(x + c * me.facing, y)
        end
    end

    -- 0x080EDFDC: the gun's animation one on; 11 ticks later, back to idle without it.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            local gun = me.related1
            if gun then
                gun.anim += 1
            end
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t >= 0 then
            return
        end
        me:set_status("using_action", false)
        me.anim = 0
        me.related1 = nil
        me:exit_attack()
    end

    return new.action {
        id = spec.id,
        state = { timer = "u16", beam = "object" },
        -- 0x080EDE0E: the phase, then the reactive-defense abort (requests 0x200, 0x400).
        update = function(me: Object, s: State)
            local step = me.step
            if step == WIND_UP then
                wind_up(me, s)
            elseif step == FIRING then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("GunSol's phase %#x reads past its table (0x080EDE38)", step))
            end
            if me:request("anti_damage_triggered") or me:request("anti_sword_triggered") then
                me:check_reactive_abort()
            end
        end,
    }
end

-- A gun in its palette (attachment rows 0x1F to 0x22).
function gunsol.gun(palette: number): AttachmentLook
    return attachment.look { sprite = asset.sprite("gun-del-sol"), palette = palette }
end

return gunsol
