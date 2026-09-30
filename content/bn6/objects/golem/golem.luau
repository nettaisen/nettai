--!strict
-- GolmHit's golem fist (effect object #0x3F, `sub_80E4DB0`) and the chips'
-- effect (instant chip effect 21, `sub_80E4FCA`). The fist leaps from the
-- user's panel to the column of the nearest of the other side's navis on
-- the user's row (Param2 0; else, or if there is none, the user's panel
-- ahead): a long hop from 95 pixels short and 40 up, then a short one from
-- 10 short and 30 up, leaving dust afterimages; it highlights its column
-- as it goes. On a panel that can take it, it lands on its column: hits on
-- the three panels, with dust, cracking its panel (Param3) and the ones
-- above and below (Param4 1); elsewhere it hits its panel alone and is
-- gone. Param1 is its palette.

local golem = {}

local SPRITE = "10-30"

-- The nearest target's column (`byte_80E4E74`, by side: required and
-- forbidden panel flags): the other side's navi's panel.
local TARGET_REQUIRE = { [0] = 0x10020, [1] = 0x10000 }
local TARGET_FORBID = { [0] = 0, [1] = 0x20 }
-- Where a panel can take the landing.
local SOLID = 0x10
-- The hops: how far short and how high each starts, and its speeds, 16.16.
local HOP_1_BACK, HOP_1_UP, HOP_1_VX, HOP_1_VZ = 0x5F_0000, 0x28_0000, 0x20_8000, 0xC_8000
local HOP_2_BACK, HOP_2_UP, HOP_2_VX, HOP_2_VZ = 0xA_0000, 0x1E_0000, 0x5_0000, 0xF_0000
local HOP_TICKS = 2
-- The landing's hits (`dword_80E50BC`): region, hit spark, collision
-- types, hit modifier.
local REGION, TARGET, SELF_TYPE, HIT_MOD = 1, 5, 0x15, 3
-- Effects: the dust where it lands, and where it can't.
local DUST, GONE = 0x34, 0x15
-- The highlighted column: region 4, around its panel.
local COLUMN = 4
-- Its dust afterimages (`sub_80E4FFC`, `sub_80E5024`).
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0x83E0, 0xE
local AFTERIMAGE_LOW = 0x4_0000
local STAY_TICKS = 0x2D

-- The actions.
local LEAP, HOP_1, HOP_2, LANDED = 0, 4, 8, 0xC

-- `sub_80E4FCA`, instant chip effect 21 (GolmHit1-3): a fist on the user's
-- panel. (It spawns at the registers' values, which its init replaces but
-- for Z.)
function golem.instant_chip(user: Object, spec: InstantChipSpec)
    local o = battle.spawn_kind("golem", Vec3.new(spec.panel_y, spec.element, spec.z), spec.params)
    if o then
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.element = spec.element
        o.alliance, o.flip = user.alliance, user.flip
        o.damage = spec.damage
        o.stamina = bit32.rshift(spec.damage, 16)
    end
end

-- `object_getClosestPanelMatchingRowFiltered`: from the back column of the
-- side `from_right` names, the first panel on row `y` that meets the
-- conditions (0 if none).
local function closest(from_right: boolean, y: number, require: number, forbid: number): number
    local x = if from_right then 6 else 1
    local step = if from_right then -1 else 1
    while true do
        if field.check(x, y, require, forbid) then
            return x
        end
        x += step
        if not field.valid(x, y) then
            return 0
        end
    end
end

-- `sub_80E4FE8`: highlight its column, on the field.
local function highlight(me: Object)
    local facing = me.facing
    for _, d in data.regions[COLUMN] or {} do
        local x, y = me.panel_x + d[1] * facing, me.panel_y + d[2]
        if x >= 1 and x <= 6 and y >= 1 and y <= 3 then
            field.highlight(x, y)
        end
    end
end

-- `sub_80E4FFC` / `sub_80E5024`: a dust afterimage of animation `anim`,
-- `dy` below it.
local function dust(me: Object, anim: number, dy: number)
    local p = me.pos
    -- (Its sprite, 10-30, as Param1 and Param2; no shadow.)
    local params = { 0x10, 0x30, anim, bit32.bxor(me.alliance, me.flip) }
    battle.afterimage(me, Vec3.new(p.x, p.y + dy, p.z), params, AFTERIMAGE_SHADER, AFTERIMAGE_TICKS)
end

-- Move by its speed: forward, and down.
local function drift(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z - me.vel.z)
end

-- `sub_80E5068`: a hit and dust on (x, y), if it can take the landing.
local function strike(me: Object, x: number, y: number)
    if not bit32.btest(field.flags(x, y), SOLID) then
        return
    end
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = REGION,
        hit_effect = 0xFF,
        target = TARGET,
        self_type = SELF_TYPE,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), DUST)
end

-- `sub_80E4EB8`: the first dust, then the first hop at once.
local function leap(me: Object)
    dust(me, 4, 0)
    me.action = HOP_1
end

-- `sub_80E4EC8`: the first hop; then the second.
local function hop_1(me: Object)
    drift(me)
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me.timer = HOP_TICKS
        me:set_coordinates_from_panel()
        local facing = me.facing
        local p = me.pos
        me.pos = Vec3.new(p.x - facing * HOP_2_BACK, p.y, HOP_2_UP)
        me.vel = Vec3.new(facing * HOP_2_VX, me.vel.y, HOP_2_VZ)
        me.anim = 3
        dust(me, 3, AFTERIMAGE_LOW)
        me.action = HOP_2
    end
end

-- `sub_80E4F18`: the second hop; then the landing.
local function hop_2(me: Object)
    drift(me)
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        return
    end
    local x, y = me.panel_x, me.panel_y
    if not bit32.btest(field.flags(x, y), SOLID) then
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.element,
            z = 0,
            region = REGION,
            hit_effect = 0xFF,
            target = TARGET,
            self_type = SELF_TYPE,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        battle.effect(me.pos, GONE)
        me:set_lifecycle("destroy")
        return
    end
    -- sub_80E5050
    strike(me, x, y)
    strike(me, x, y - 1)
    strike(me, x, y + 1)
    -- (And a camera shake: presentation only.)
    battle.play_sound(0xC0)
    battle.play_sound(0x10D)
    if me:param(3) ~= 0 then
        field.crack(x, y)
        if me:param(4) == 1 then
            field.crack(x, y - 1)
            field.crack(x, y + 1)
        end
    end
    me.timer = STAY_TICKS
    me.action = LANDED
end

-- `sub_80E4FB6`: stay, then go.
local function landed(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E4E84`.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == LEAP then
        leap(me)
        hop_1(me)
    elseif a == HOP_1 then
        hop_1(me)
    elseif a == HOP_2 then
        hop_2(me)
    elseif a == LANDED then
        landed(me)
    else
        error(string.format("golem action %#x reads past its table", a))
    end
    highlight(me)
    me:update_sprite_even_paused()
end

-- `sub_80E4DD0`: over the target column, short of it and up.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if me:param(2) == 0 then
        local side = me.alliance
        local from_right = bit32.bxor(bit32.bxor(side, me.flip), 1) == 0
        local x = closest(from_right, me.panel_y, TARGET_REQUIRE[side], TARGET_FORBID[side])
        if x == 0 then
            x = me.panel_x + me.facing
        end
        me.panel_x = x
    end
    me:set_coordinates_from_panel()
    local facing = me.facing
    local p = me.pos
    me.pos = Vec3.new(p.x - facing * HOP_1_BACK, p.y, HOP_1_UP)
    me.vel = Vec3.new(facing * HOP_1_VX, me.vel.y, HOP_1_VZ)
    me.timer = HOP_TICKS
    sprite.palette = me:param(1)
    me.visible = true
    battle.play_sound(0x188)
    me.anim, me.anim_loaded = 4, 0
    sprite:set_animation(4)
    sprite:step()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80E4DB0`.
function golem.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return golem
