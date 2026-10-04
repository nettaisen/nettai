--!strict
-- FireHit's fist and the chips' effect, as BN5 and BN6 share them (BN6's
-- attack object #0x5B, `sub_80CFCF8`, and instant chip effect 8,
-- `sub_80CFE08`; BN5's #0x5B, 0x080D131C, and effect 9). The effect
-- looks ahead of the user, column by column along its row, for a column
-- with something of the other side in it (any row); a fist appears on
-- each such row, on that column or at most three panels ahead of the
-- user. The fist, hidden, highlights its panel for 10 ticks, then
-- strikes: a hit on its panel and 37 ticks of its animation.
--
-- Each game makes its kind with `fist.make_kind(id, look)` (its sprite,
-- sound, spark and collision types).

local regions = require("../regions")

local fist = {}

export type State = {
    -- The strike's hit modifier (the chip's second parameter).
    hit_mod: number,
}

-- What the effect looks for (`off_80CFEB8`, by the user's side): the other
-- side's navis and objects.
local TARGETS = { [0] = 0x04000000, [1] = 0x08000000 }

-- The strike (`dword_80CFEC0`): one panel, the fire spark, an attack that
-- hits navis.
local STRIKE_Z = 0x10_0000

-- The actions.
local WARN, STRIKE = 0, 4

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
local function strike(me: Object, look: FireHitFistLook)
    if me.phase_init == 0 then
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = STRIKE_Z,
            region = regions.single,
            hit_effect = look.spark,
            target = look.target,
            self_type = look.self_type,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = (me.state :: State).hit_mod,
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
local function tick(me: Object, look: FireHitFistLook)
    local a = me.action
    if a == WARN then
        warn(me)
    elseif a == STRIKE then
        strike(me, look)
    else
        error(string.format("FireHit's fist action %#x reads past its table", a))
    end
end

-- `sub_80CFD1C`.
local function init(me: Object, look: FireHitFistLook)
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = false
    me:set_coordinates_from_panel()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    battle.play_sound(look.sound)
    me:set_lifecycle("update")
    tick(me, look)
end

-- A fist kind (`id`) of `look`.
function fist.make_kind(id: string, look: FireHitFistLook): Kind
    return define.kind {
        id = id,
        pool = "attack",
        state = { hit_mod = "u8" },
        -- `sub_80CFCF8`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
            end
            me:update_sprite()
        end,
    }
end

-- `sub_80CFDF2` and `sub_801155A`: a fist (of `kind`, a game's) of
-- `owner`'s on (x, y), its hit with the damage word `damage` and modifier
-- `hit_mod`. (It spawns at the registers' values, which its init replaces
-- but for Z.)
function fist.spawn(kind: Kind, owner: Object, x: number, y: number, element: number, z: number, hit_mod: number, damage: number): Object?
    local o = battle.spawn(kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner;
        (o.state :: State).hit_mod = hit_mod
    end
    return o
end

-- `sub_80CFE5C`: a fist on column `at` of each row where column `probe` has
-- one of the other side's; how many.
local function strike_column(user: Object, probe: number, at: number, hit_mod: number, spec: InstantChipSpec, kind: Kind): number
    local targets = TARGETS[user.alliance]
    local n = 0
    for y = 1, 3 do
        if bit32.btest(field.flags(probe, y), targets) then
            fist.spawn(kind, user, at, y, spec.element, spec.z, hit_mod, spec.damage)
            n += 1
        end
    end
    return n
end

-- `sub_80CFE08`, instant chip effect 8: an `instant` hook whose fists (of
-- `kind`, a game's) hit with modifier `hit_mod`. The columns ahead of the user while its row's
-- panel there is on the field; the fists' column follows for the first
-- three.
function fist.instant(hit_mod: number, kind: Kind): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local dir = user.facing
        local probe = user.panel_x + dir
        local at = probe
        local misses = 0
        while field.valid(probe, user.panel_y) do
            if strike_column(user, probe, at, hit_mod, spec, kind) ~= 0 then
                return
            end
            probe += dir
            misses += 1
            if misses < 3 then
                at += dir
            end
        end
    end
end

return fist
