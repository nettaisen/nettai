--!strict
-- A sword wave (the original's attack object #0x96, `sub_80D807C`): what
-- SlashCross's charged slashes (action 0x41) send along the row. It hits
-- what it meets, blinks out for 10 ticks after a hit, and ends off the
-- field or once it has passed its reach of panel centers.
--
-- What it is comes with its variant (a `SwordWaveVariant` record, a row
-- of the original's `byte_80D7F4C` by the first parameter): its
-- collision, look, speed and reach. `sword_wave.waves` are the rows by
-- name; each charged slash (./slashes) names its own.

local collision = require("../../../../rules/collision")
local regions = require("../../../../lib/regions")
local statuses = require("../../../../rules/status")

export type State = {
    variant: SwordWaveVariant,
    -- ExtraVars+0: it has moved once (the first move doesn't count a
    -- panel center).
    started: boolean,
    -- Panel centers left to pass (the original's fourth parameter).
    reach: number,
    -- It isn't drawn (the original's second parameter).
    hidden: boolean,
}

local sword_wave = {}

-- The actions: moving, blinking out after a hit.
local MOVING, BLINKING = 0, 1

-- `sub_800E6E8`: moving from `old` to `new` passed `center` (reaching it
-- counts, starting on it doesn't).
local function passed(new: number, old: number, center: number): boolean
    if new > old then
        return center > old and center <= new
    end
    return center > new and center <= old
end

-- The end: hidden, no region, destroyed next update.
local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80D8172`: resolve its hits; blink out after a hit, or move on,
-- counting the panel centers it passes; register again.
local function tick(me: Object)
    local s = me.state :: State
    local v = s.variant
    local c = me.collision
    c:remove()
    me:hit_spark()
    if me.action ~= MOVING then
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            finish(me)
        else
            me.visible = not bit32.btest(me.timer, 2)
        end
    elseif c.hit_flags ~= 0 then
        c:set_region(nil)
        me.timer = 10
        me.action = BLINKING
    else
        local center = field.center(me.panel_x, me.panel_y)
        local p = me.pos
        local old = p.x
        me.pos = Vec3.new(old + me.vel.x, p.y, p.z)
        local new = me.pos.x
        local over = false
        if not s.started then
            s.started = true
        elseif passed(new, old, center) then
            local left = s.reach - 1
            s.reach = left
            over = left <= 0
        end
        if over then
            finish(me)
        else
            me:set_panel_from_coordinates()
            me:update_collision_panels()
            if not field.valid(me.panel_x, me.panel_y) then
                finish(me)
            end
        end
    end
    c:present()
    if v.highlight then
        me:highlight_collision_panels()
    end
    if v.steps then
        me:update_sprite()
    end
end

-- `sub_80D80B4`: its variant's look, speed, reach and registration.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local s = me.state :: State
    local v = s.variant
    -- (The game keeps the variant's record in the first related slot, over
    -- the spawner.)
    me.related1 = nil
    local sprite = me.sprite
    sprite:load(v.sprite)
    sprite.shadow = if v.ground_shadow then "ground" else "with_sprite"
    me.anim = v.anim
    me.anim_loaded = 0xFF
    sprite.palette = v.palette
    if not s.hidden then
        me.visible = true
    end
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    s.reach = v.reach
    me:set_panel_from_coordinates()
    me.vel = Vec3.new(int.mul32(me.facing, v.speed), me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(v.collision.is, v.collision.hits, v.hit_mod)
    local c = me.collision
    c:set_hit_effect(nil)
    c:set_region(v.region)
    local status = v.status
    if status then
        c.status_base = status
    end
    local bug = v.bug
    if bug then
        c.bugs = bug + (v.bug_arg or 0) * 0x100
    end
    c:present()
    me:update_sprite()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D807C`: its lifecycle; then a variant that animates restarts its
-- animation after the last frame and steps.
local function update(me: Object)
    local animates = (me.state :: State).variant.animates
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if animates then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me.anim_loaded = 0xFF
        end
        me:update_sprite()
    end
end

sword_wave.kind = define.kind {
    id = "slashcross/sword-wave",
    pool = "attack",
    state = { variant = "record:sword-wave-variant", started = "bool", reach = "u8", hidden = "bool" },
    update = update,
}

-- A wave's variant, as a record the wave keeps.
function sword_wave.variant(v: SwordWaveVariant): SwordWaveVariant
    return define.record("sword-wave-variant", v)
end

-- `sub_80D8222`: a wave of `variant` and `owner`'s on (x, y) with the
-- damage word `damage`; `hidden`, it isn't drawn (the original's second
-- parameter). (Its position is its spawner's registers, of which only the
-- Z, 0 here, survives its init.)
function sword_wave.spawn(
    owner: Object,
    variant: SwordWaveVariant,
    x: number,
    y: number,
    element: number,
    damage: number,
    hidden: boolean?
): Object?
    local o = battle.spawn(sword_wave.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.variant = variant
        s.hidden = hidden == true
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80D8242`: the same, running while dimmed.
function sword_wave.spawn_while_dimmed(
    owner: Object,
    variant: SwordWaveVariant,
    x: number,
    y: number,
    element: number,
    damage: number,
    hidden: boolean?
): Object?
    local o = sword_wave.spawn(owner, variant, x, y, element, damage, hidden)
    if o then
        o.run_while_dimmed = true
    end
    return o
end

-- ---- The waves of the swords SlashCross charges ----------------------------

local SLASH = { is = collision.slash, hits = collision.hits_navis }
local SWORD_SLASH, BIG_SLASH = asset.sprite("sword-slash"), asset.sprite("big-slash")

-- A wave of the rows' usual make: a slash that reaches navis, 8 pixels a
-- tick, over 8 panel centers.
local function wave(v: {
    region: Region,
    sprite: SpriteAsset,
    anim: number,
    palette: number?,
    hit_mod: number?,
    status: Status?,
    reach: number?,
    animates: boolean?,
    steps: boolean?,
    bug: number?,
    bug_arg: number?,
}): SwordWaveVariant
    return sword_wave.variant {
        collision = SLASH,
        hit_mod = v.hit_mod or 3,
        region = v.region,
        sprite = v.sprite,
        anim = v.anim,
        palette = v.palette or 0,
        reach = v.reach or 8,
        speed = 0x8_0000,
        status = v.status,
        animates = v.animates,
        steps = v.steps,
        bug = v.bug,
        bug_arg = v.bug_arg,
    }
end

-- The waves, by the sword the slash is of (the original's rows: the
-- charged sword chip's subtype, then the charge's own). `charge`: the
-- charged shot's own short wave, whose sprite also steps after each
-- update; `moon`: the moon blade's ring, whose hits give a bug (code 0x18,
-- argument 1); `unused`: row 7, a sword no chip is.
sword_wave.waves = {
    sword = wave { region = regions.single, sprite = SWORD_SLASH, anim = 2 },
    wide = wave { region = regions.wide, sprite = SWORD_SLASH, anim = 0 },
    long = wave { region = regions.long, sprite = SWORD_SLASH, anim = 1 },
    wide_blade = wave { region = regions.wide, sprite = SWORD_SLASH, anim = 0, palette = 5 },
    long_blade = wave { region = regions.long, sprite = SWORD_SLASH, anim = 1, palette = 5 },
    life = wave { region = regions.wide_long, sprite = BIG_SLASH, anim = 0 },
    dark = wave { region = regions.wide_long, sprite = BIG_SLASH, anim = 0, palette = 10 },
    unused = wave { region = regions.wide_long, sprite = BIG_SLASH, anim = 0 },
    muramasa = wave { region = regions.long, sprite = SWORD_SLASH, anim = 1, palette = 6 },
    fighter = wave { region = regions.line, sprite = SWORD_SLASH, anim = 3 },
    cross = wave { region = regions.x, sprite = asset.sprite("cross-slash"), anim = 0, animates = true },
    double_dream = wave { region = regions.wide_long, sprite = BIG_SLASH, anim = 0, hit_mod = 1 },
    fire = wave { region = regions.wide, sprite = SWORD_SLASH, anim = 0, palette = 1 },
    aqua = wave { region = regions.wide, sprite = SWORD_SLASH, anim = 0, palette = 2 },
    -- (It paralyzes.)
    elec = wave { region = regions.wide, sprite = SWORD_SLASH, anim = 0, palette = 3, hit_mod = 1, status = statuses.paralyze_90 },
    bamboo = wave { region = regions.wide, sprite = SWORD_SLASH, anim = 0, palette = 4, hit_mod = 1 },
    charge = wave { region = regions.wide, sprite = asset.sprite("charged-slash"), anim = 0, reach = 1, steps = true },
    moon = wave {
        region = regions.ring,
        sprite = asset.sprite("moon-blade"),
        anim = 0,
        animates = true,
        bug = 0x18,
        bug_arg = 1,
    },
    double_dream_second = wave { region = regions.wide_long, sprite = BIG_SLASH, anim = 0 },
}

return sword_wave
