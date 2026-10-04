--!strict
-- GyroSoul's tornado (BN5's attack object 0x1E, 0x080C8194; BN5's own, its BN6 counterpart `sub_80C7F40` out of a
-- netbattle's reach): a whirlwind (sprite 10-0C) lying along the three panels ahead of MegaMan, which hits them
-- three times, eight ticks apart: all three, then the far two, then the farthest (0x080C82FC), each hit lasting
-- eight ticks. It highlights the three panels while it blows, and goes after the third hit or when the battle ends.

local collision = require("../../../../rules/collision")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")

local tornado = {}

export type State = {
    -- Param2: it lies on the ground (none: 15 pixels below it, GyroSoul's).
    grounded: boolean,
    -- +0x22: the hits so far.
    hits: number,
}

local SPRITE = asset.sprite("bn5:gyrosoul-tornado")
local SOUND = asset.sound("bn5:gyrosoul-tornado")
-- Its sprite sits this far ahead of its panel, and this far down unless it lies on the ground (16.16).
local AHEAD, BELOW = 38 * 0x1_0000, 15 * 0x1_0000
-- The ticks between hits, and a hit's.
local BETWEEN, LASTS = 8, 8
-- By hit, whether it reaches each of the three panels (0x080C82FC).
local PATTERN = { { true, true, true }, { false, true, true }, { false, false, true } }
-- A hit's hit modifier: the pattern's byte, which the spawner leaves in r7 (`object_spawnCollisionRegion`'s).
local HIT_MOD = 1

-- 0x080C82A2: hit `n` (1 to 3): an attack of no spark on each panel the pattern names, at the tornado's height.
local function hit(me: Object, n: number)
    local x = me.panel_x
    for i = 1, 3 do
        if PATTERN[n][i] then
            local box = battle.hitbox(me, {
                panel_x = x,
                panel_y = me.panel_y,
                element = me.element,
                z = me.pos.z,
                region = regions.single,
                hit_effect = nil,
                target = collision.hits_navis,
                self_type = collision.attack_a0,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = HIT_MOD,
            })
            if box then
                box.timer = LASTS
            end
        end
        x += me.facing
    end
end

-- 0x080C8248: the three panels highlighted; a hit every eight ticks from the first; gone after the third.
local function blow(me: Object)
    local s = me.state :: State
    panels.highlight_region(me.panel_x, me.panel_y, regions.line, me.facing)
    if me.phase_init == 0 then
        s.hits = 0
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t > 0 then
        return
    end
    me.timer = BETWEEN
    s.hits += 1
    if s.hits > 3 then
        me:set_lifecycle("destroy")
        return
    end
    hit(me, s.hits)
end

-- 0x080C81BC: its sprite (no shadow) at animation 0 on its panel, facing its side's way, ahead of the panel and
-- (unless on the ground) below it; its sound.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me:set_coordinates_from_panel()
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.facing * AHEAD), p.y, if s.grounded then 0 else -BELOW)
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

tornado.kind = define.kind {
    id = "bn5:gyrosoul/tornado",
    pool = "attack",
    state = { grounded = "bool", hits = "u16" },
    -- 0x080C8194: its state; then its sprite steps unless the battle is dimmed.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            -- 0x080C8222: gone when the battle is over.
            if battle.over() then
                me:set_lifecycle("destroy")
            elseif me.action == 0 then
                blow(me)
            else
                error(string.format("GyroSoul's tornado's action %#x reads past its table (0x080C8244)", me.action))
            end
        else
            me:destroy()
            return
        end
        if not battle.dimmed() then
            me:update_sprite()
        end
    end,
}

-- 0x080C828C: the tornado on panel (x, y) for `owner`, on its side, Null, of damage word `damage`.
function tornado.spawn(owner: Object, x: number, y: number, damage: number): Object?
    -- (Its position is the spawner's registers until its init places it.)
    local o = battle.spawn(tornado.kind, Vec3.new(y, 0, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return tornado
