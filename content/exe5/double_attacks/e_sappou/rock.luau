--!strict
-- E-SAPPOU's rock (the DS's T1 0x44, 0x0209665C): it falls from 192 pixels onto a panel beside an enemy, lands with a
-- shake, waits its turn, and becomes a copy of ShadowMan (08-08) holding a sword (the attachment table's row 3 in its
-- animation 8), who sweeps the eight panels around him with the moon blade (EXE5's attack object #0x69, MoonBld's,
-- as the DS has it), blinks and goes. It faces the enemy: the rock behind it is turned round.

local attachment = require("../../objects/attachment")
local effects = require("../../lib/effects")
local collision = require("../../rules/collision")
local common_blade = require("@exelib/moonbld/blade")

local rock = {}

export type State = {
    -- The word its spawner gave it (+0x60): its Colonel's flag for it, cleared as it goes.
    colonel: Object?,
    slot: number,
    -- The sword the copy holds (+0x64).
    held: Object?,
}

type ColonelState = { out: { boolean } }

local FALL_FROM, GRAVITY = 0xC00000, -0xC000
local ROCK, SHADOWMAN = asset.sprite("rock"), asset.sprite("shadowman")
local ANIM_ROCK = 1
local SOUND_LAND = asset.sound("land")
-- Its landing's shake (0, 20 ticks).
local SHAKE, SHAKE_TICKS = 0, 0x14
-- The copy: appearing (animation 3) in a puff of dust 30 pixels up, 4 ticks.
local ANIM_APPEAR, APPEAR_TICKS, DUST_Z = 3, 4, 0x1E0000
-- The slash: animation 5, the sword, the swing's sound; the blade on the 10th tick; blinking from the 11th; gone after
-- the 41st.
local ANIM_SLASH, SWORD_ANIM, BLADE_AT, BLINK_FROM, SLASH_TICKS = 5, 8, 9, 0xA, 0x28
local SWORD = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local SOUND_SWING = asset.sound("sword-swing")
-- Action 0x10 (0x020967E4: no action of the rock's starts it): animation 4, its sword let go, 4 ticks, gone.
local ANIM_LEAVE, LEAVE_TICKS = 4, 4

-- The copy's moon blade: EXE5's attack object #0x69 (MoonBld's, chips/moonbld: sprite 10-38, EXE5's rows 7 and 5) as the
-- DS has it, which E-SAPPOU's spawner runs while dimmed (Param3 1): it sweeps on its first tick, its hits resolve while
-- dimmed (@exelib/moonbld/blade's `ds`).
rock.blade = common_blade.make_kind("e-sappou/blade", {
    sprite = asset.sprite("moon-blade"),
    is = collision.slash,
    hits = collision.hits_navis,
    ds = true,
})

local function ran_out(me: Object): boolean
    local t = me.timer - 1
    me.timer = t
    return t <= 0
end

-- 0x020966C4: it falls; on the ground, the landing's sound.
local function fall(me: Object)
    local p, v = me.pos, me.vel
    local vz = int.i32(v.z + GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    local z = int.i32(p.z + vz)
    me.pos = Vec3.new(p.x, p.y, z)
    if z > 0 then
        return
    end
    battle.play_sound(SOUND_LAND)
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_action(4)
end

-- 0x020966F0: the shake, then its wait (Param1).
local function wait(me: Object, ticks: number)
    if me.phase_init == 0 then
        battle.shake_camera(SHAKE, SHAKE_TICKS)
        me.timer = ticks
        me.phase_init = 4
    end
    if ran_out(me) then
        me:set_action(8)
    end
end

-- 0x0209671C: the copy of ShadowMan comes, in a puff of dust.
local function copy(me: Object)
    if me.phase_init == 0 then
        local sprite = me.sprite
        sprite:load(SHADOWMAN)
        sprite.shadow = "ground"
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.anim, me.anim_loaded = ANIM_APPEAR, 0xFF
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + DUST_Z)), effects.dust)
        me.timer = APPEAR_TICKS
        me.phase_init = 4
    end
    if ran_out(me) then
        me:set_action(0xC)
    end
end

-- 0x0221076C (EXE5's 0x080D32B0, with the DS's Param3): the moon blade on the copy's panel, its element 0, the copy's side,
-- facing and damage word, riding on it and running while dimmed (Param3 1), its hits MoonBld's (Param1 0). Its position
-- is what the registers held: the panel's row in X.
local function blade(me: Object)
    local o = battle.spawn(rock.blade, Vec3.new(me.panel_y, 0, 0))
    if o then
        o.panel_x, o.panel_y = me.panel_x, me.panel_y
        o.element = 0
        o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        o.alliance, o.flip = me.alliance, me.flip
        o.related1 = me
        o.run_while_dimmed = true
    end
end

-- 0x02096774: the slash.
local function slash(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = ANIM_SLASH
        attachment.attach(me, SWORD, "held", { anim = SWORD_ANIM, while_dimmed = true })
        battle.play_sound(SOUND_SWING)
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer
    if t == BLADE_AT then
        blade(me)
    end
    me.visible = true
    if t >= BLINK_FROM and bit32.btest(t, 2) then
        me.visible = false
    end
    t += 1
    me.timer = t
    if t > SLASH_TICKS then
        s.held = nil
        me:set_lifecycle("destroy")
    end
end

-- 0x020967E4.
local function leave(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_LEAVE, 0xFF
        s.held = nil
        me.timer = LEAVE_TICKS
        me.phase_init = 4
    end
    if ran_out(me) then
        me:set_lifecycle("destroy")
    end
end

local function run(me: Object, wait_ticks: number)
    local a = me.action
    if a == 0 then
        fall(me)
    elseif a == 4 then
        wait(me, wait_ticks)
    elseif a == 8 then
        copy(me)
    elseif a == 0xC then
        slash(me)
    elseif a == 0x10 then
        leave(me)
    else
        error(string.format("E-SAPPOU's rock: action %#x reads past its table (0x02131E48)", a))
    end
end

-- A rock's own: its wait (Param1).
rock.kind = new.kind {
    id = "e-sappou/rock",
    pool = "actor",
    state = { colonel = "object", slot = "u8", held = "object", wait = "u8" },
    update = function(me: Object)
        local s = me.state :: State & { wait: number }
        local l = me.lifecycle
        if l == "init" then
            -- 0x02096670: the rock (10-02, animation 1), shown, on its panel 192 pixels up.
            local sprite = me.sprite
            sprite:load(ROCK)
            sprite.shadow = "ground"
            me.anim, me.anim_loaded = ANIM_ROCK, 0xFF
            me.visible = true
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, FALL_FROM)
            me:set_lifecycle("update")
            run(me, s.wait)
        elseif l == "update" then
            run(me, s.wait)
        else
            -- 0x02096810: its Colonel's flag cleared.
            local colonel = s.colonel
            if colonel then
                (colonel.state :: ColonelState).out[s.slot + 1] = false
            end
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x02096824: a rock on (x, y) for `colonel`, its element 0, of his side and facing (turned round when `behind`), with
-- his damage word, waiting `wait` ticks after it lands, its flag his rock `slot`.
function rock.spawn(colonel: Object, x: number, y: number, wait: number, behind: boolean, slot: number): Object?
    local o = battle.spawn(rock.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = 0
    o.alliance = colonel.alliance
    o.flip = bit32.bxor(colonel.flip, if behind then 1 else 0)
    o:set_damage_word(bit32.bor(colonel.damage, bit32.lshift(colonel.stamina, 16)))
    o.related1 = colonel
    local s = o.state :: State & { wait: number }
    s.colonel, s.slot, s.wait = colonel, slot, wait;
    (colonel.state :: ColonelState).out[slot + 1] = true
    return o
end

return rock
