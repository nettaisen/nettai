--!strict
-- BN5's CrakBom, ParaBom and ResetBom bomb (attack object #0x24, 0x080C91A4: BN6's #0x24, `sub_80C882C`, the same
-- code, which no BN6 chip throws): thrown three panels ahead in 35 ticks (0x080C9406), it lands and hits the column
-- there (region 4) with its chip's hit, a burst on each panel of it; off solid ground it ends in a puff. What
-- differs is its kind (the record's Param1): 1 CrakBom's hit modifier 3, cracking the column; 2 ParaBom's
-- paralysis; 3 ResetBom's bug 0xFB (MegaMan loses his body programs), each its burst's palette and its sound.
-- Kinds 4 and up would light the column up as it flies, and 5 crack only its panel (no chip throws those).

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local statuses = require("../../rules/status")
local trajectory = require("../../lib/trajectory")

local crakbom = {}

export type State = {
    -- Its kind (Param1).
    which: number,
    -- Its flight's ticks (Param2, 35).
    ticks: number,
    -- The panel it aims at (Param3 and Param4).
    target_x: number,
    target_y: number,
}

-- 0x080C91C8: sprite 0c-02 with a ground shadow, animation 7.
local SPRITE = asset.sprite("bn5:bomb")
local ANIM = 7
local FLIGHT_TICKS, GRAVITY = 35, -0x3000
-- Effect 0x14: a puff of smoke.
local PUFF = define.effect { sprite = asset.sprite("bn5:puff"), anim = 2, palette = 0 }
-- Effects 0x50 to 0x53 (0x080C93E8, by the kind's low two bits): the burst in palettes 0 to 3.
local BURST = asset.sprite("bn5:bomb-burst")
local BURSTS = {
    [0] = define.effect { sprite = BURST, anim = 0, palette = 0 },
    [1] = define.effect { sprite = BURST, anim = 0, palette = 1 },
    [2] = define.effect { sprite = BURST, anim = 0, palette = 2 },
    [3] = define.effect { sprite = BURST, anim = 0, palette = 3 },
}
-- Its landing's sound (0x080C932C, by the kind's low two bits).
local SOUNDS = {
    [0] = asset.sound("bn5:hit-bomb-1"),
    [1] = asset.sound("bn5:panel-crack"),
    [2] = asset.sound("bn5:panel-poison"),
    [3] = asset.sound("bn5:panel-poison"),
}
-- Its hit by kind (0x080C93C8: hit modifier, status, bug and argument bytes): 0x3, 0x3, 0x1000 (paralysis, status
-- 0x10), 0xFB0003, then the same again.
type Hit = { hit_mod: number, status: Status?, bug: number }
local HITS: { [number]: Hit } = {
    [0] = { hit_mod = 3, bug = 0 },
    [1] = { hit_mod = 3, bug = 0 },
    [2] = { hit_mod = 0, status = statuses.paralyze_90, bug = 0 },
    [3] = { hit_mod = 3, bug = 0xFB },
    [4] = { hit_mod = 3, bug = 0 },
    [5] = { hit_mod = 3, bug = 0 },
    [6] = { hit_mod = 0, status = statuses.paralyze_90, bug = 0 },
    [7] = { hit_mod = 3, bug = 0xFB },
}

-- A puff where it is, and gone.
local function puff(me: Object)
    battle.effect(me.pos, PUFF)
    me:set_lifecycle("destroy")
end

-- 0x080C9354: the landing: a burst on each panel of the column, the column's hit; a kind 1 cracks it, a kind 5 its
-- panel (0x080C93EC, cracking whatever the panel).
local function land(me: Object, s: State)
    local x, y = me.panel_x, me.panel_y
    battle.region_effects(x, y, regions.wide, me.alliance, BURSTS[bit32.band(s.which, 3)], 0)
    local hit = HITS[s.which]
    if hit == nil then
        error(string.format("CrakBom's bomb kind %d reads past its hits", s.which))
    end
    local word = int.u32(me.damage + me.stamina * 0x10000)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0,
        region = regions.wide,
        -- None.
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = hit.hit_mod,
        status = hit.status,
        bug = hit.bug,
        bug_arg = 0,
    })
    if s.which == 5 then
        field.crack(x, y)
    elseif s.which == 1 then
        field.crack(x, y)
        field.crack(x, y + 1)
        field.crack(x, y - 1)
    end
end

-- 0x080C9240: fly to the target panel; land there.
local function fly(me: Object, s: State)
    if me.phase_init == 0 then
        local tx, ty = field.center(s.target_x, s.target_y)
        me.vel = trajectory.in_ticks(me.pos, tx, ty, 0, s.ticks, GRAVITY)
        me.timer = s.ticks
        me.phase_init = 4
    end
    -- (On its eighth tick before landing, 0x080E9FD2 spawns effect object 0x83 at the target panel with battle flag
    -- 0x40, which a netbattle never sets: not ported, bn5-map.md §15.6.)
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z + v.z)
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
        me:set_panel_from_coordinates()
        -- (Kinds 4 and up light up the column every other four ticks.)
        if bit32.btest(t, 4) and s.which >= 4 then
            field.highlight(s.target_x, s.target_y)
            field.highlight(s.target_x, s.target_y - 1)
            field.highlight(s.target_x, s.target_y + 1)
        end
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    if not field.solid(me.panel_x, me.panel_y) then
        puff(me)
        return
    end
    land(me, s)
    battle.play_sound(SOUNDS[bit32.band(s.which, 3)])
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- 0x080C920A: the battle over, a puff; else fly, then go.
local function tick(me: Object)
    if battle.over() then
        puff(me)
        return
    end
    local a = me.action
    if a == 0 then
        fly(me, me.state :: State)
    elseif a == 4 then
        -- 0x080C9330.
        me:set_lifecycle("destroy")
    else
        error(string.format("CrakBom's bomb action %#x reads past its table", a))
    end
end

crakbom.kind = define.kind {
    id = "bn5:crakbom/bomb",
    pool = "attack",
    state = { which = "u8", ticks = "u8", target_x = "u8", target_y = "u8" },
    -- 0x080C91A4.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080C91C8.
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "ground"
            me.visible = true
            me.anim, me.anim_loaded = ANIM, ANIM
            sprite:set_animation(ANIM)
            sprite.palette = 0
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_panel_from_coordinates()
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080C9406: a thrower of kind `which` bombs, three panels ahead of the user.
function crakbom.thrower(which: number): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local o = battle.spawn(crakbom.kind, pos)
            if o then
                o.alliance, o.flip = user.alliance, user.flip
                o.element = 0
                o:set_damage_word(damage)
                local s = o.state :: State
                s.which = which
                s.ticks = FLIGHT_TICKS
                s.target_x, s.target_y = user.panel_x + 3 * user.facing, user.panel_y
            end
            return o
        end,
    }
end

return crakbom
