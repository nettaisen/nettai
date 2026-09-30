--!strict
-- FireHit's fist (attack object #0x5B, `sub_80CFCF8`) and the chips' effect
-- (instant chip effect 8, `sub_80CFE08`). The effect looks ahead of the
-- user, column by column along its row, for a column with something of the
-- other side in it (any row); a fist appears on each such row, on that
-- column or at most three panels ahead of the user. The fist, hidden,
-- highlights its panel for 10 ticks, then strikes: a hit on its panel and
-- 37 ticks of its animation. Param2 is the hit's modifier.

local fire_hit = {}

local SPRITE = "14-1a"

-- What the effect looks for (`off_80CFEB8`, by the user's side): the other
-- side's navis and objects.
local TARGETS = { [0] = 0x04000000, [1] = 0x08000000 }

-- The strike (`dword_80CFEC0`): region, hit spark, collision types.
local REGION, HIT_SPARK, TARGET, SELF_TYPE = 1, 1, 5, 4

-- The actions.
local WARN, STRIKE = 0, 4

-- `sub_80CFDF2` and `sub_801155A`: a fist of `owner`'s on (x, y). (It
-- spawns at the registers' values, which its init replaces but for Z.)
function fire_hit.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    params: { number },
    damage: number
): Object?
    local o = battle.spawn_kind("fire-hit", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80CFE5C`: a fist on column `at` of each row where column `probe` has
-- one of the other side's; how many.
local function strike_column(user: Object, probe: number, at: number, spec: InstantChipSpec): number
    local targets = TARGETS[user.alliance]
    local n = 0
    for y = 1, 3 do
        if bit32.btest(field.flags(probe, y), targets) then
            fire_hit.spawn(user, at, y, spec.element, spec.z, spec.params, spec.damage)
            n += 1
        end
    end
    return n
end

-- `sub_80CFE08`, instant chip effect 8 (FireHit1-3): the columns ahead of
-- the user while its row's panel there is on the field; the fists' column
-- follows for the first three.
function fire_hit.instant_chip(user: Object, spec: InstantChipSpec)
    local dir = user.facing
    local probe = user.panel_x + dir
    local at = probe
    local misses = 0
    while field.valid(probe, user.panel_y) do
        if strike_column(user, probe, at, spec) ~= 0 then
            return
        end
        probe += dir
        misses += 1
        if misses < 3 then
            at += dir
        end
    end
end

-- `sub_80CFD80`: hidden, the panel highlighted every other four ticks,
-- for 10 ticks.
local function warn(me: Object)
    if me.phase_init == 0 then
        me.timer = 0xA
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me.action = STRIKE
        me.phase, me.phase_init = 0, 0
    end
    if bit32.btest(me.timer, 4) then
        field.highlight(me.panel_x, me.panel_y)
    end
end

-- `sub_80CFDB0`: the hit, then 37 ticks of the fist.
local function strike(me: Object)
    if me.phase_init == 0 then
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = 0x10_0000,
            region = REGION,
            hit_effect = HIT_SPARK,
            target = TARGET,
            self_type = SELF_TYPE,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = me:param(2),
        })
        me.visible = true
        me.anim, me.anim_loaded = 0, 0xFF
        me.timer = 0x25
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80CFD64`.
local function tick(me: Object)
    local a = me.action
    if a == WARN then
        warn(me)
    elseif a == STRIKE then
        strike(me)
    else
        error(string.format("FireHit's fist action %#x reads past its table", a))
    end
end

-- `sub_80CFD1C`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = false
    me:set_coordinates_from_panel()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    battle.play_sound(0xED)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80CFCF8`.
function fire_hit.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return fire_hit
