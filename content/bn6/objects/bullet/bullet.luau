--!strict
-- The bullet (the original's attack object #0x12, `sub_80C6946`): the
-- Vulcans' and Spreaders' shot, SpoutCross's charged shot, ColArmy's and
-- ColForce's soldiers'. An unseen probe that steps a panel a tick from where it
-- appears; on the first thing it would hit it bursts there: its variant's
-- region is hit once (the tick after it finds the target), with an effect
-- shown over the region or at the bullet. What differs between its uses
-- is its variant (`bullet.variant { ... }`, the original's `byte_80C68D4`
-- rows). See docs/engine/shot-chips.md §4.1.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local statuses = require("../../rules/status")

local bullet = {}

export type State = {
    variant: BulletVariant,
}

-- A bullet variant.
function bullet.variant(v: BulletVariant): BulletVariant
    return define.record("bullet-variant", v)
end

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80C6A08`: a panel forward; off the field it ends.
local function step(me: Object)
    me.panel_x = me.panel_x + me.facing
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        finish(me)
    end
end

-- `sub_80C6A50`: the burst: its hit over the variant's region on the panel
-- it found (`sub_80C6AB8`: an attack of its own element now, which hits
-- with the variant's modifier and status), the effect, the sound.
local function burst(me: Object, v: BulletVariant)
    me:reset_collision_types(collision.attack, collision.hits_navis, v.hit_mod)
    me.collision.element = bit32.band(me.element, 0xF)
    me.collision.secondary_element = bit32.band(me.element, 0xF0)
    local status = v.status
    if status then
        me.collision.status_base = status
    end
    local c = me.collision
    c:set_region(v.region)
    if v.single then
        battle.effect(me.pos, v.effect, bit32.bxor(me.alliance, me.flip))
    else
        battle.region_effects(c.panel_x, c.panel_y, v.region, me.alliance, v.effect, me.pos.z)
    end
    if v.sound then
        battle.play_sound(v.sound)
    end
    me.phase, me.phase_init = 4, 0
end

-- `sub_80C69AC`: the hits against it resolve and spark; the first time it
-- finds something (its DEAD status a latch) it bursts there.
local function tick(me: Object)
    local v = (me.state :: State).variant
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 and not me:status("dead") then
        me:set_status("dead", true)
        me.action = 4
        me.phase, me.phase_init = 0, 0
    end
    if me.action == 0 then
        step(me)
    elseif me.phase == 0 then
        burst(me, v)
    else
        -- `sub_80C6AAC`: its hit landed; done.
        c:set_region(nil)
        me:set_lifecycle("destroy")
    end
    c:present()
end

-- `sub_80C6964`: on its panel, unseen, a probe (§1 of shot-chips.md: it
-- finds what an attack would hit without hurting it, element null).
local function init(me: Object)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.probe, collision.hits_navis, 0)
    me.collision.element = 0
    me.collision:present()
    me:set_lifecycle("update")
end

bullet.kind = define.kind {
    id = "bullet",
    pool = "attack",
    state = { variant = "record:bullet-variant" },
    -- `sub_80C6946`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80C6ADA`: a bullet of `variant` from `owner` on panel (x, y) at
-- height `z` (16.16), with `element` and the damage word; nil if the
-- attack pool is full. It takes the owner's side and flip, runs later
-- this tick, right after its spawner, and keeps its Z.
function bullet.fire(owner: Object, x: number, y: number, z: number, variant: BulletVariant, element: number, damage: number): Object?
    local o = battle.spawn(bullet.kind, Vec3.new(0, 0, z))
    if o then
        (o.state :: State).variant = variant
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- The variants (the original's rows: region, hit modifier, effect,
-- status, sound; rows 0xC, 0xE, 0x10..0x12 show their effect once, at the
-- bullet).
local SPLASH = define.effect { sprite = asset.sprite("spout-splash"), anim = 0, palette = 0 }
local HIT = define.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 0 }
local HIT_BRIGHT = define.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 1 }
local GLINT = define.effect { sprite = asset.sprite("form-change"), anim = 1, palette = 0 }
local SPARKLE = define.effect { sprite = asset.sprite("colonel-effect"), anim = 3, palette = 0 }
local SPREAD = asset.sound("h-burst")
local SPLASHING = asset.sound("spout-ball")
-- (The fire rows' sound is the one the target squares make as they move.)
local FIRE = asset.sound("target-move")

local function v(region: Region, hit_mod: number, effect: Effect, sound: Sound?, more: { single: boolean?, status: Status? }?): BulletVariant
    local m: { single: boolean?, status: Status? } = more or {}
    return bullet.variant { region = region, hit_mod = hit_mod, effect = effect, sound = sound, single = m.single, status = m.status }
end
local FORK = regions.fork
local ROWS: { [number]: BulletVariant } = {
    [0x00] = v(regions.long, 0, effects.flash, SPREAD),
    [0x01] = v(FORK, 0, effects.flash, SPREAD),
    [0x02] = v(regions.wide, 0, effects.flash, SPREAD),
    [0x03] = v(regions.square, 0, effects.flash, SPREAD),
    [0x04] = v(regions.long, 0, SPLASH, SPLASHING),
    [0x05] = v(FORK, 0, SPLASH, SPLASHING),
    [0x06] = v(regions.wide, 0, SPLASH, SPLASHING),
    [0x07] = v(regions.square, 3, SPLASH, SPLASHING),
    [0x08] = v(regions.long, 0, effects.shell_burst, FIRE),
    [0x09] = v(FORK, 0, effects.shell_burst, FIRE),
    [0x0A] = v(regions.wide, 0, effects.shell_burst, FIRE),
    [0x0B] = v(regions.square, 0, effects.shell_burst, FIRE),
    [0x0C] = v(regions.long, 1, HIT, nil, { single = true }),
    [0x0D] = v(regions.long, 3, SPLASH, SPLASHING),
    [0x0E] = v(regions.long, 1, GLINT, nil, { single = true }),
    [0x0F] = v(regions.square, 1, SPARKLE, SPREAD),
    [0x10] = v(regions.long, 1, HIT_BRIGHT, nil, { single = true }),
    [0x11] = v(regions.long, 1, HIT, nil, { single = true, status = statuses.paralyze_150 }),
    [0x12] = v(regions.long, 1, HIT, nil, { single = true, status = statuses.paralyze_90 }),
}
bullet.variants = {
    -- The Vulcans' (row 0xC) and SuprVulc's (0x10): the panel found and
    -- the one behind it, staggering.
    vulcan = ROWS[0x0C],
    super_vulcan = ROWS[0x10],
    -- The Spreaders' (row 3): the 3x3 around the panel found.
    spreader = ROWS[0x03],
    -- SpoutCross's charged shot (row 0xD).
    spout = ROWS[0x0D],
    -- The patch card puffball's bubble spread (row 7): the 3x3 around the
    -- panel found, with hit modifier 3.
    bubble = ROWS[0x07],
    -- ColArmy's soldiers' (row 0x11) and ColForce's (0x12): staggering,
    -- with their statuses.
    col_army = ROWS[0x11],
    col_force = ROWS[0x12],
}

-- The variant of row `n`, for the callers that still name one by number
-- (the weapons' attack parameters); past the table the game reads on into
-- the routine's code: an error.
function bullet.by_number(n: number): BulletVariant
    local r = ROWS[n]
    if not r then
        error(string.format("bullet row %#x reads past byte_80C68D4", n))
    end
    return r
end

return bullet
