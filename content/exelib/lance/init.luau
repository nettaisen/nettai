--!strict
-- A lance, as EXE5 and EXE6 share it (EXE6's attack object #0x6F,
-- `sub_80D2460`; EXE5's #0x6F; EXE4's #0x4F, 0x080D73C8): Lance's
-- three lances (chip.luau) thrust out of the far column's panels, one per
-- row. Each appears 64 pixels past its panel, slides back 8 pixels a tick,
-- hits the panel it is over on its seventh tick, and blinks out over 20
-- ticks; EXE4's is an attack the whole time it shows instead (its look's
-- `attack`). Its sprite never animates. A lance spawned with Param1 set
-- (none of EXE6's chips) takes the second palette. Each game's lance is a
-- kind of its look (`lance.make_kind`: its sprite and sound, or its attack,
-- EXE4's); each chip states its palette and its hit (`lance.hit`: what the
-- original reads by its Param1, EXE5's DrkLance's its own).

local regions = require("./regions")

local lance = {}

export type State = {
    -- Its palette (0, or 1 with Param1 set), and its chip's hit (none: an
    -- attack the whole time, EXE4's).
    palette: number,
    hit: LanceHit?,
}

-- A chip's lance's hit.
function lance.hit(h: LanceHit): LanceHit
    return new.record("lance-hit", h)
end


-- How far past its panel it appears, in pixels, and how fast it slides
-- back, 16.16.
local AHEAD, SPEED = 0x40, 0x8_0000

-- The actions.
local THRUST, BLINK = 0, 4

-- A lance kind (`id`) of `look`.
function lance.make_kind(id: string, look: LanceLook): Kind
    -- `sub_80D2514`: slide back; the hit on the seventh tick (an attack's
    -- registration moves along instead, EXE4's 0x080D74C8); after eight,
    -- blink.
    local function thrust(me: Object)
        local p = me.pos
        me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
        me:set_panel_from_coordinates()
        if look.attack then
            me:update_collision_panels()
        elseif me.timer == 2 then
            local hit = (me.state :: State).hit
            if not hit then
                error("a lance with no hit of its chip's (sub_80D2514 reads its Param1's row)")
            end
            battle.hitbox(me, {
                panel_x = me.panel_x,
                panel_y = me.panel_y,
                element = me.element,
                z = me.pos.z,
                region = regions.single,
                hit_effect = hit.spark,
                target = hit.hits,
                self_type = hit.is,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = hit.hit_mod,
                bug = hit.bug or 0,
                bug_arg = hit.bug_arg or 0,
            })
        end
        local left = me.timer - 1
        me.timer = left
        if left ~= 0 then
            return
        end
        me.timer = 0x14
        me.action = BLINK
        me.phase, me.phase_init = 0, 0
    end

    -- `sub_80D255A`: blink (hidden every other pair of ticks), then go (an
    -- attack's region ending first).
    local function blink(me: Object)
        local left = me.timer - 1
        me.timer = left
        if left == 0 then
            if look.attack then
                me.collision:set_region(nil)
            end
            me.visible = false
            me:set_lifecycle("destroy")
            return
        end
        if not bit32.btest(left, 2) then
            me.visible = false
        end
    end

    -- `sub_80D24F0`; an attack's hits resolve and spark first, the first
    -- that lands ends its region, and it registers again after (EXE4's
    -- 0x080D7482).
    local function tick(me: Object)
        me.visible = true
        if look.attack then
            local c = me.collision
            c:remove()
            me:hit_spark()
            if c.hit_flags ~= 0 then
                c:set_region(nil)
            end
        end
        local a = me.action
        if a == THRUST then
            thrust(me)
        elseif a == BLINK then
            blink(me)
        else
            error(string.format("lance action %#x reads past its table", a))
        end
        if look.attack then
            me.collision:present()
        end
    end

    -- `sub_80D2480`. False when an attack's lance found no collision
    -- record (it is gone).
    local function init(me: Object): boolean
        me.visible = true
        local sprite = me.sprite
        sprite:load(look.sprite)
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite.shadow = "ground"
        sprite:set_flip(bit32.bxor(bit32.bxor(me.alliance, me.flip), 1))
        me:set_coordinates_from_panel()
        local facing = me.facing
        local p = me.pos
        me.pos = Vec3.new(p.x + facing * AHEAD * 0x1_0000, p.y, p.z)
        me.vel = Vec3.new(-facing * SPEED, me.vel.y, me.vel.z)
        me:set_panel_from_coordinates()
        me.timer = 8
        sprite.palette = (me.state :: State).palette
        -- (EXE4's 0x080D7452: an attack's registration, its spark by its
        -- element.)
        local attack = look.attack
        if attack then
            if not me:create_collision() then
                me:free()
                return false
            end
            me:setup_collision(attack.is, attack.hits, attack.hit_mod)
            me.collision:set_hit_effect(attack.spark(me.element))
            me.collision:present()
        end
        battle.play_sound(look.sound)
        me:set_lifecycle("update")
        tick(me)
        return true
    end

    return new.kind {
        id = id,
        pool = "attack",
        state = { palette = "u8", hit = "record:lance-hit" },
        -- `sub_80D2460`; an attack's panel highlighted while its region
        -- stands (EXE4's 0x080D73D4).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                if not init(me) then
                    return
                end
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
                return
            end
            if look.attack and me.collision.region then
                field.highlight(me.panel_x, me.panel_y)
            end
        end,
    }
end

-- `sub_80D2580` and `sub_801155A`: a lance of `owner`'s on (x, y) in
-- palette `palette`, with the damage word `damage`. (It spawns at the
-- registers' values, which its init replaces but for Z.) `side_only`: the
-- owner's side alone, not its facing nor itself (EXE4's 0x080D7522).
-- `hit`: its chip's (none: EXE4's, an attack the whole time).
function lance.spawn_kind(
    kind: Kind,
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    palette: number,
    damage: number,
    side_only: boolean?,
    hit: LanceHit?
): Object?
    local o = battle.spawn(kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        if not side_only then
            o.flip = owner.flip
            o.related1 = owner
        end
        local st = o.state :: State
        st.palette, st.hit = palette, hit
    end
    return o
end

-- `sub_80D2596`: lances of `kind` with Param1 `palette` on the far column,
-- rows 1 to 3, at height `z`; the column by the user's side and facing, or
-- its side alone with `side_only` (EXE4's 0x080D7540, action 0x20's
-- variant 6).
function lance.thrust(
    kind: Kind,
    user: Object,
    element: number,
    z: number,
    palette: number,
    damage: number,
    side_only: boolean?,
    hit: LanceHit?
)
    local facing = if side_only then user.alliance else bit32.bxor(user.alliance, user.flip)
    local x = 6 - facing * 5
    for y = 1, 3 do
        lance.spawn_kind(kind, user, x, y, element, z, palette, damage, side_only, hit)
    end
end

-- Instant chip effect 4 (EXE6's, EXE5's): `lance.thrust` at the user's
-- height, the chip's lances in `palette` with its `hit`.
function lance.instant_of(kind: Kind, palette: number, hit: LanceHit): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        lance.thrust(kind, user, spec.element, spec.z, palette, spec.damage, false, hit)
    end
end

return lance
