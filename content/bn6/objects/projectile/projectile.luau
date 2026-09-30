--!strict
-- The projectile (attack object #0, `sub_80C4E58`): the shot the buster,
-- the charged shot, the cannons and many chips fire. It flies forward one
-- panel every two ticks (the first step on the tick after it appears) and
-- ends on the first thing it hits or when it leaves the field. Its kind,
-- its first parameter, is data (data.objects.projectiles): its collision,
-- element, spark, look, status and bug, what its hit does to the panel,
-- and whether it bursts or climbs. Its second parameter is a row it also
-- moves on its first step (the spread's side shots).
--
-- Callers fire it with lib/projectile.luau. See
-- docs/engine/objects-and-player.md §B8.

local element = require("../../lib/element")

-- (What it is fired with, `ProjectileShot`, is declared for the pack in
-- types.d.luau.)

local projectile = {}

-- Ticks it waits on a panel after the one it enters on (Timer2).
local WAIT = 1

-- A bursting kind's burst (`sub_80C5014`, `sub_80C5050`): its sound, the
-- effect it shows over the panels, and the hit (which reaches as far as
-- the effects but the panel the projectile hit itself).
local BURST_SOUND, BURST_EFFECT = 0xC3, 0
local HIT_EFFECTS, HIT_REGION = 0x0F, 0x10
local EXIT_REGION = 0x11
local BURST_SELF, BURST_TARGET, BURST_HIT_MOD = 0x04, 0x05, 3

-- `sub_80C4FFE`: a projectile from `owner` on panel (x, y). It takes the
-- owner's side and flip, and runs later this tick, right after its
-- spawner. (Its X and Y are its spawner's registers until its init, the
-- same tick, places it.)
function projectile.spawn(owner: Object, x: number, y: number, shot: ProjectileShot): Object?
    local row = int.u8(shot.row or 0)
    local o = battle.spawn_kind("projectile", Vec3.new(0, 0, shot.z), { shot.kind, row, 0, 0 })
    if o then
        o.panel_x, o.panel_y = x, y
        o.damage = shot.damage
        o.stamina = bit32.rshift(shot.damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- Its kind's record (`off_80C4C78`): a kind past the table reads on into
-- the routine's code.
local function kind_of(me: Object): ProjectileKind
    local n = me:param(1)
    local k = data.objects.projectiles[n]
    if k == nil then
        error(string.format("projectile kind %#x reads past off_80C4C78", n))
    end
    return k
end

-- `sub_80C5090`: a climbing kind sits a pixel further down the field and
-- a pixel higher on each panel it enters than on the last (its Z keeps
-- what it had).
local function climb(me: Object, k: ProjectileKind)
    if k.climbs then
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y + 0x10000, p.z + 0x10000)
    end
end

-- `object_clearCollisionRegion`, then the end (the next update frees it).
local function finish(me: Object)
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C5050` and `sub_80C5014`: the burst on (x, y): the effect over
-- the `effects` region's panels and a hit over `region`'s. (And a camera
-- shake: presentation only.)
local function burst(me: Object, x: number, y: number, effects: number, region: number)
    battle.play_sound(BURST_SOUND)
    battle.region_effects(x, y, effects, me.alliance, BURST_EFFECT)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        region = region,
        hit_effect = 0xFF,
        target = BURST_TARGET,
        self_type = BURST_SELF,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = BURST_HIT_MOD,
    })
end

-- What the hit does (`loc_80C4F70`): a bursting kind bursts around its
-- panel; then the panel cracks, breaks or changes type, as the kind says.
local function hit(me: Object, k: ProjectileKind)
    local x, y = me.panel_x, me.panel_y
    if k.bursts then
        burst(me, x, y, HIT_EFFECTS, HIT_REGION)
    end
    local h = k.hit_panel
    if h == nil then
        return
    end
    if h.effect == "crack" then
        field.crack(x, y)
    elseif h.effect == "break" then
        field.break_panel(x, y)
    elseif field.solid(x, y) then
        local kind = if me.alliance == 0 then h.left_side else h.right_side
        field.set_type(x, y, kind :: PanelType)
    end
end

-- Wait out the panel, then step forward (and a row, the first time). Off
-- the field it ends: a bursting kind bursts over the last two columns it
-- crossed.
local function fly(me: Object, k: ProjectileKind)
    if me.phase_init == 0 then
        me.timer = me.timer2
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me.panel_x = me.panel_x + me.facing
    me.panel_y = me.panel_y + int.i8(me:param(2))
    me:set_param(2, 0)
    local x, y = me.panel_x, me.panel_y
    if not field.valid(x, y) then
        if k.bursts then
            burst(me, x - 2 * me.facing, y, EXIT_REGION, EXIT_REGION)
        end
        finish(me)
        return
    end
    me:set_coordinates_from_panel()
    climb(me, k)
    me:update_collision_panels()
    me.phase_init = 0
end

-- `sub_80C4F02`: the hits against it resolve and spark; it ends when the
-- battle is over or it hit something, else it flies on.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        finish(me)
    elseif c.hit_flags ~= 0 then
        hit(me, kind_of(me))
        finish(me)
    else
        fly(me, kind_of(me))
    end
    c:present()
end

-- `sub_80C4E7C`: its look, element and collision from its kind, on its
-- panel. (The game overwrites its link to its owner with its record's
-- address, so the owner is forgotten.)
local function init(me: Object)
    local k = kind_of(me)
    me.related1 = nil
    local sprite = k.sprite
    if sprite then
        me.sprite:load(sprite)
        -- sprite_noShadow
        me.sprite.shadow = "with_sprite"
        me.visible = true
        me.anim = k.anim or 0
        me.anim_loaded = 0xFF
    end
    me.element = element.byte(k.element, k.secondary)
    me:set_coordinates_from_panel()
    climb(me, k)
    me.timer2 = WAIT
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(k.self_type, k.target_type, k.hit_mod)
    local c = me.collision
    c.hit_effect = k.hit_effect
    local status, bug = k.status or 0, k.bug or 0
    if status ~= 0 then
        c.status_base = status
    end
    if bug ~= 0 then
        c.bugs = bit32.bor(bug, bit32.lshift(k.bug_arg or 0, 8))
    end
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C4E58`.
function projectile.update(me: Object)
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

return projectile
