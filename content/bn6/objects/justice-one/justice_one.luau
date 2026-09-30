--!strict
-- JustcOne's fist (attack object #0xAE, `sub_80DB304`) and the chip's
-- effect (instant chip effect 19, `sub_80DB4B4`): a Break fist falls from
-- 256 pixels up and behind onto the other side's middle panel (column 5
-- facing right, 2 facing left), 16 pixels a tick for 16 ticks. It hits
-- what it falls through. On a panel that can take it, it lands: a burst,
-- the panels around it crack (region 0xF), two wide hits of 100 (regions
-- 0xA and 0xC) that land while dimmed too, and nine puffs of dust. Then it
-- stays, blinking, for 30 ticks (unless its panel is gone).

local justice_one = {}

local SPRITE = "0c-62"
-- Break (secondary element 0x10).
local BREAK = 0x10
-- Its own collision types and hit modifier (`dword_80DB558`), and hit
-- spark.
local SELF_TYPE, TARGET_TYPE, HIT_MOD, HIT_SPARK = 0x15, 5, 3, 0xA
-- How far behind and above it starts, and how fast it falls (both ways),
-- 16.16.
local START, SPEED = 0x100_0000, 0x10_0000
local FALL_TICKS = 0x10
-- What it can land on (`dword_80DB418`): a panel with a body or object on
-- it, or a solid one.
local LANDS = 0x0F800010
-- The panels it cracks, and the landing hits' regions, hit spark,
-- collision types, damage and hit modifier (`dword_80DB564`,
-- `dword_80DB56C`).
local CRACKS = 0xF
local WAVES = { 0xA, 0xC }
local WAVE_TARGET, WAVE_SELF = 5, 4
local WAVE_DAMAGE, WAVE_HIT_MOD = 0x64, 3
-- The dust (effect 0) around its panel (`byte_80DB540`: dx facing
-- forward, dy).
local DUST = { { -1, -1 }, { -1, 0 }, { -1, 1 }, { 0, -1 }, { 0, 0 }, { 0, 1 }, { 1, -1 }, { 1, 0 }, { 1, 1 } }
-- The effects: the landing's burst, and where its panel is gone.
local BURST, GONE = 5, 0x14
local STAY_TICKS = 0x1E

-- The actions.
local FALL, STAY = 0, 4

-- `sub_80DB474` and `sub_801155A`: the fist of `owner`'s on (x, y). (It
-- spawns at the registers' values, which its init replaces.)
function justice_one.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    params: { number },
    damage: number
): Object?
    local o = battle.spawn_kind("justice-one", Vec3.new(y, element, z), params)
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

-- `sub_80DB4B4`, instant chip effect 19 (JustcOne): the other side's
-- middle panel, Break.
function justice_one.instant_chip(user: Object, spec: InstantChipSpec)
    local x = 5 - bit32.bxor(user.alliance, user.flip) * 3
    justice_one.spawn(user, x, 2, BREAK, spec.z, spec.params, spec.damage)
end

-- `sub_80DB324`.
local function init(me: Object)
    me.visible = true
    -- sub_8011504(0xC62, 0x15050300)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        -- The game goes on without one, writing through a null pointer.
        error("JustcOne's fist without a collision slot (sub_80DB324 doesn't check)")
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local facing = me.facing
    me.vel = Vec3.new(facing * SPEED, me.vel.y, SPEED)
    local p = me.pos
    me.pos = Vec3.new(p.x - facing * START, p.y, START)
    me.timer = FALL_TICKS
    battle.play_sound(0xC4)
    local c = me.collision
    c.hit_effect = HIT_SPARK
    c.region = 0
    me.lifecycle = "update"
end

-- `sub_80DB4CE`: the landing's hits (running while dimmed) and dust. (And
-- a camera shake: presentation only.)
local function land(me: Object)
    local damage = bit32.band(me.damage, 0xF000) + WAVE_DAMAGE
    for _, region in WAVES do
        local box = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = 0,
            z = 0,
            region = region,
            hit_effect = 0xFF,
            target = WAVE_TARGET,
            self_type = WAVE_SELF,
            damage = damage,
            stamina = me.stamina,
            hit_mod = WAVE_HIT_MOD,
        })
        -- sub_80C53A6
        if box then
            box.run_while_dimmed = true
        end
    end
    local facing = me.facing
    for _, d in DUST do
        local x, y = field.center(me.panel_x + d[1] * facing, me.panel_y + d[2])
        battle.effect(Vec3.new(x, y, 0), 0)
    end
end

-- `sub_80DB48A`: crack the panels of region `region` around (x, y).
local function crack_region(x: number, y: number, region: number)
    for _, d in data.regions[region] or {} do
        field.crack(x + d[1], y + d[2])
    end
end

-- `sub_80DB3B4`: fall; then land, if the panel can take it.
local function fall(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        local p = me.pos
        me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z - me.vel.z)
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    if bit32.btest(field.flags(me.panel_x, me.panel_y), LANDS) then
        battle.effect(me.pos, BURST)
        battle.play_sound(0x70)
        crack_region(me.panel_x, me.panel_y, CRACKS)
        land(me)
        me.collision.region = 1
    end
    me.action = STAY
    me.phase, me.phase_init = 0, 0
end

-- `sub_80DB41C`: stay 30 ticks, blinking; gone at once where its panel
-- is.
local function stay(me: Object)
    local c = me.collision
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    if not field.solid(me.panel_x, me.panel_y) then
        battle.effect(me.pos, GONE)
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if me.phase_init == 0 then
        me.timer = STAY_TICKS
        me.phase_init = 4
    end
    me.visible = true
    if bit32.btest(me.timer, 2) then
        me.visible = false
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        c.region = 0
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DB376`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local a = me.action
    if a == FALL then
        fall(me)
    elseif a == STAY then
        stay(me)
    else
        error(string.format("JustcOne's fist action %#x reads past its table", a))
    end
    me:update_sprite()
    c:present()
end

-- `sub_80DB304`.
function justice_one.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return justice_one
