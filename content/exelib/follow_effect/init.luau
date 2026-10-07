--!strict
-- A follow effect, as EXE6 and EXE5 share it (EXE6's effect object #0x31,
-- `sub_80E3D68`; EXE5's #0x31, 0x080E3E3C): a sprite shown for its owner
-- (related1), and what it does by its look (a row of the game's looks and
-- routines):
--
-- - "none": nothing more;
-- - "slide": slides out beside its owner until the owner's related1 is
--   gone, then plays out;
-- - "flash" (DeltaRay's burst): shows for its timer (60 ticks, which its
--   spawner sets again), switching to animation 1 below 55;
-- - "while_in_action": lasts while its owner stays in the action it was
--   spawned in;
-- - "riding": the same, riding on its owner;
-- - "timed": the same, 30 ticks at most;
-- - "with_idle_owner" (EXE4's look 5, MetalMan's: 0x080E5AC8): hidden, but while its owner is in its animation 0
--   shown as its owner is, on the owner's panel a pixel up, with the owner's blending and mosaic; once the fight is
--   on it no longer runs while paused.
--
-- The original's looks that last through an action name the owner's action
-- by number, each the action that spawns them; here the effect keeps the
-- action its owner was in when it came. Those stand still while dimmed
-- (their HP's bit 0 holds the sprite). Its flip and a palette to add are
-- its spawner's. Purely visual, but it holds an effect slot. EXE6's copies
-- its owner's visibility each tick; EXE5's doesn't. Each game makes its kind
-- (`follow_effect.make_kind`: its id, its slide's sound, whether it copies
-- the visibility). See docs/engine/chips.md §3.6.36.

local follow_effect = {}

-- A look (a row of the game's looks table and its routine): FollowEffectLook (types.d.luau).
export type Behavior = FollowEffectBehavior
export type Look = FollowEffectLook

-- A game's follow effect kind: its id, its slide's sound (a game without
-- the slide gives none), and whether it is shown as its owner is.
export type KindSpec = { id: string, slide_sound: Sound?, copies_visibility: boolean }

export type State = {
    look: Look,
    -- The game's flip value and the palette added (its spawner's).
    flip: number,
    palette_add: number,
    -- The action its owner was in when it came.
    action: Action?,
}

-- "slide"'s steps, pixels a tick (EXE6's `dword_80E3EB0`).
local SLIDE = { [0] = 2, 3, 2, 0 }

-- How long "flash" shows, and when it switches; how long "timed" lasts at
-- most.
local FLASH_TICKS, FLASH_SWITCH, TIMED_TICKS = 0x3C, 0x37, 0x1E

-- A look, as a record (a spawner's).
function follow_effect.look(l: Look): Look
    return new.record("follow-effect-look", l)
end

-- A game's follow effect kind.
function follow_effect.make_kind(spec: KindSpec): Kind
    local function look_of(me: Object): Look
        return (me.state :: State).look
    end

    -- "while_in_action", "riding", "timed": the owner is still in its action.
    local function owner_stays(me: Object): boolean
        return (me.related1 :: Object):navi_action() == (me.state :: State).action
    end

    -- Those stand still while dimmed (HP 1 holds the sprite).
    local function frozen(me: Object): boolean
        me.hp = 1
        if battle.dimmed() then
            return true
        end
        me.hp = 0
        return false
    end

    -- `sub_80E3E38`: slide out beside the owner (16 pixels ahead or behind by
    -- its flip), until the owner's related1 is gone; then play out. (On its
    -- first tick the game compares what the sound call left in r0 with 1; the
    -- port takes it to be another value, so that tick only plays out.)
    local function slide(me: Object)
        local sound = spec.slide_sound
        if not sound then
            error(string.format("%s has no slide (its game's looks have none)", spec.id))
        end
        local s = me.state :: State
        local first = me.action == 0
        if first then
            me.action = 1
            me.timer2 = 0
            battle.play_sound(sound)
        end
        if first or me.action ~= 1 then
            if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
                me:set_lifecycle("destroy")
            end
            return
        end
        local owner = me.related1
        if owner then
            local dx = if s.flip ~= 0 then -0x10 else 0x10
            local p, q = me.pos, owner.pos
            me.pos = Vec3.new(q.x + dx * 0x10000, q.y, p.z)
        end
        local t = me.timer2
        if t ~= 4 then
            local step = SLIDE[t]
            if s.flip == 0 then
                step = -step
            end
            local p = me.pos
            me.pos = Vec3.new(p.x + step * 0x10000, p.y, p.z)
            me.timer2 = t + 1
        end
        if (me.related1 :: Object).related1 == nil then
            me.anim = 3
            me.action = 2
        end
    end

    -- `sub_80E3EB4` (EXE5's 0x080E4024): shows for its timer, animation 1
    -- below 55.
    local function flash(me: Object)
        if me.action == 0 then
            me.action = 4
            me.timer = FLASH_TICKS
        end
        if me.timer < FLASH_SWITCH then
            me.anim = 1
        end
        local left = me.timer - 1
        me.timer = left
        if left <= 0 then
            me:set_lifecycle("destroy")
        end
    end

    -- `sub_80E3EDC`: while the owner is in its action.
    local function while_in_action(me: Object)
        if frozen(me) then
            return
        end
        if not owner_stays(me) then
            me:set_lifecycle("destroy")
        end
    end

    -- `sub_80E3EFC`, `sub_80E3F2A`: riding on the owner while it is in its
    -- action.
    local function riding(me: Object)
        if not frozen(me) and not owner_stays(me) then
            me:set_lifecycle("destroy")
        end
        me.pos = (me.related1 :: Object).pos
    end

    -- `sub_80E3F6C`: 30 ticks at most, while the owner is in its action.
    local function timed(me: Object)
        me.hp = 1
        if me.action == 0 then
            me.action = 4
            me.timer = TIMED_TICKS
        end
        if battle.dimmed() then
            return
        end
        me.hp = 0
        if owner_stays(me) then
            local left = me.timer - 1
            me.timer = left
            if left > 0 then
                return
            end
        end
        me:set_lifecycle("destroy")
    end

    -- EXE4's 0x080E5AC8: with its owner while the owner stands in its animation 0.
    local function with_idle_owner(me: Object)
        if battle.fighting() then
            me.run_while_paused = false
        end
        me.visible = false
        local owner = me.related1 :: Object
        if owner.anim ~= 0 then
            return
        end
        me:copy_visibility(owner)
        me.panel_x, me.panel_y = owner.panel_x, owner.panel_y
        me:set_coordinates_from_panel()
        me.sprite.alpha = owner.sprite.alpha
        me.sprite.mosaic = owner.sprite.mosaic
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y - 0x10000, p.z)
    end

    local function none(_me: Object) end

    local BEHAVIORS: { [Behavior]: (Object) -> () } = {
        none = none,
        slide = slide,
        flash = flash,
        while_in_action = while_in_action,
        riding = riding,
        timed = timed,
        with_idle_owner = with_idle_owner,
    }

    -- `sub_80E3DE0`: what its look does, then (EXE6's) shown as the owner is.
    local function tick(me: Object)
        BEHAVIORS[look_of(me).behavior](me)
        if spec.copies_visibility then
            me:copy_visibility(me.related1 :: Object)
        end
    end

    -- `sub_80E3D90`: its look, then its first update.
    local function init(me: Object)
        local s = me.state :: State
        local l = s.look
        local sprite = me.sprite
        sprite:load(l.sprite)
        sprite.shadow = "with_sprite"
        me.anim = l.anim
        me.anim_loaded = l.anim
        sprite:set_animation(l.anim)
        sprite.palette = l.palette + s.palette_add
        sprite:set_flip(s.flip)
        me.visible = true
        me.hp = 0
        me:set_lifecycle("update")
        tick(me)
    end

    return new.kind {
        id = spec.id,
        pool = "effect",
        state = { look = "record:follow-effect-look", flip = "u8", palette_add = "u8", action = "action" },
        -- `sub_80E3D68`: its sprite steps unless held (HP bit 0).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
            if bit32.band(me.hp, 1) == 0 then
                me:update_sprite()
            end
        end,
    }
end

-- `sub_80E3FB4`: a follow effect of `kind` and `look` for `owner` at `pos`,
-- with the game's flip value `flip` and `palette_add`.
function follow_effect.spawn(kind: Kind, owner: Object, pos: Vec3, look: Look, flip: number, palette_add: number?): Object?
    local o = battle.spawn(kind, pos)
    if o then
        o.related1 = owner
        local s = o.state :: State
        s.look, s.flip, s.palette_add = look, flip, palette_add or 0
        if look.behavior == "while_in_action" or look.behavior == "riding" or look.behavior == "timed" then
            local a = owner:navi_action()
            if type(a) ~= "table" then
                error("a follow effect that lasts through its owner's action follows no content action")
            end
            s.action = a :: Action
        end
    end
    return o
end

-- `sub_80E3FC4`: end it (its next update frees it).
function follow_effect.stop(o: Object)
    o:set_lifecycle("destroy")
end

return follow_effect
