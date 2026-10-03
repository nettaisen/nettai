--!strict
-- A barrier's visual, as BN5 and BN6 share it (BN6's effect object #7, `sub_80E0AD4`; BN5's #7, 0x080E0AA0; BN6's
-- docs/engine/dimming-chips.md §3.3): it sits on its navi at the look's attach point and follows the navi's barrier
-- (the collision data's): up, it shows; worn down, it goes down (and comes back up when the barrier grows back); with
-- the barrier gone, it goes. The navi links it (`barrier_visual`, AIData+0x60), and hides and shows it with itself
-- (its `shown`).
--
-- Each game makes its own kind (`visual.make`): BN6's is blown away when wind pops the barrier (`sub_80E0C74`'s
-- popped branch, `sub_80E0D38`) and comes back up only on a navi that isn't bubbled (`sub_80E0CAA`); BN5's has
-- neither (0x080E0C38, 0x080E0C5C: no blown action, no bubble status).

local element = require("@common/element")

local visual = {}

export type State = {
    barrier: Barrier,
    -- Shown with its navi (the original's +0x0C).
    shown: boolean,
    -- The hit modifier that popped the barrier (its ExtraVars+8).
    pop_hit_mod: number,
}

export type Spec = {
    -- The kind's id.
    id: string,
    -- BN6's: blown away when popped, and back up only on a navi that isn't bubbled.
    blows: boolean,
}

export type Visual = {
    kind: Kind,
    -- `sub_80E0D98`: `b`'s visual on `navi`, which links it.
    spawn: (navi: Object, b: Barrier) -> Object?,
}

-- Its actions.
local UP, DOWN, BLOWN = 0, 4, 8
-- Blown away: 8 pixels a tick back (or forward, pushed that way: hit modifier bits 0x14) and 2.5 up the field, for
-- 20 ticks.
local BLOWN_SPEED, BLOWN_RISE, BLOWN_TICKS = -0x80000, -0x28000, 0x14
local PUSHED_FORWARD = 0x14
-- LifeAur's aura's part 6 (`timer_2000000`'s address as the parts word).
local PART_6 = 0x2000000

local function owner_of(me: Object): Object
    local owner = me.related1
    if not owner then
        error("a barrier's visual has no navi (sub_80E0B8C reads through a null pointer)")
    end
    return owner
end

-- The navi forgets it, if it is the one the navi links.
local function unlink(me: Object)
    local owner = me.related1
    if owner and owner.barrier_visual == me then
        owner.barrier_visual = nil
    end
end

-- Seen unless the viewer is blind to its side, shown with its navi, and its navi on the field; `hide_only`: only
-- the hiding half (the update's second pass).
local function visibility(me: Object, owner: Object, hide_only: boolean)
    if not hide_only then
        me.visible = true
    end
    me:hide_from_blind()
    if not (me.state :: State).shown then
        me.visible = false
    end
    if not field.valid(owner.panel_x, owner.panel_y) then
        me.visible = false
    end
end

local function go(me: Object, action: number)
    me:set_action(action)
end

-- `sub_80E0D38`: blown away from its navi (the next animation), for 20 ticks.
local function blown(me: Object)
    if me.phase_init == 0 then
        local vx = int.mul32(me.facing, BLOWN_SPEED)
        if bit32.btest((me.state :: State).pop_hit_mod, PUSHED_FORWARD) then
            vx = int.i32(-vx)
        end
        me.vel = Vec3.new(vx, BLOWN_RISE, me.vel.z)
        me.anim = me.anim + 1
        me.timer = BLOWN_TICKS
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me.visible = false
    unlink(me)
    me:free()
end

-- `sub_801A7CC`: `b` on `navi`'s collision data.
function visual.raise_barrier(navi: Object, b: Barrier)
    navi:raise_barrier {
        behavior = b.behavior,
        hp = b.hp,
        threshold = b.threshold,
        timer = b.timer,
        weak_element = if b.weak then element.byte(b.weak) else 0,
    }
end

-- `sub_80E0DC0`: the visual is told to end (state 8: it frees itself at its next update, its link left as it is).
function visual.finish(v: Object)
    v:set_lifecycle("destroy")
end

function visual.make(spec: Spec): Visual
    local blows = spec.blows

    -- `sub_80E0C74`: up while the navi links it and its barrier holds HP; down when it doesn't; (BN6's) blown away
    -- when popped.
    local function up(me: Object, owner: Object)
        if owner.barrier_visual == nil then
            return go(me, DOWN)
        end
        local c = owner.collision
        local barrier = c.barrier
        if barrier == "none" then
            go(me, DOWN)
        elseif blows and barrier == "popped" then
            (me.state :: State).pop_hit_mod = c.barrier_pop_hit_mod
            go(me, BLOWN)
        elseif c.barrier_hp == 0 then
            go(me, DOWN)
        end
    end

    -- `sub_80E0CAA`: going down (the next animation); back up when the barrier has HP again (and, BN6's, its navi
    -- isn't bubbled); hidden at the end of the animation while the barrier stands; gone at it once the barrier is.
    local function down(me: Object, owner: Object)
        local b = (me.state :: State).barrier
        if me.phase_init == 0 then
            me.anim = me.anim + 1
            if b.down_sound then
                battle.play_sound(b.down_sound)
            end
            me.phase_init = 4
        end
        local c = owner.collision
        local last_frame = bit32.btest(me.sprite.frame_flags, 0x80)
        if c.barrier ~= "none" then
            if c.barrier_hp ~= 0 and not (blows and owner:status("bubbled")) then
                me.anim = me.anim - 1
                me.visible = true
                battle.play_sound(b.sound)
                go(me, UP)
            elseif last_frame then
                me.visible = false
            end
            return
        end
        if last_frame then
            unlink(me)
            me:free()
        end
    end

    -- `sub_80E0B8C`: visibility; on its navi (unless blown away: then only the attach point's offset adds up), 2
    -- pixels up the field and lower; its action; visibility's hiding half again.
    local function tick(me: Object)
        local owner = owner_of(me)
        visibility(me, owner, false)
        local b = (me.state :: State).barrier
        if not (blows and me.action == BLOWN) then
            local p = owner.pos
            me.pos = Vec3.new(p.x, p.y - 0x20000, p.z - 0x20000)
        end
        local ax, ay = owner:attach_point(b.look.attach_point)
        local p = me.pos
        me.pos = Vec3.new(p.x + ax * 0x10000, p.y, p.z + ay * 0x10000)
        local a = me.action
        if a == UP then
            up(me, owner)
        elseif a == DOWN then
            down(me, owner)
        elseif blows and a == BLOWN then
            blown(me)
        else
            error(string.format("a barrier's visual's action %#x reads past off_80E0C68", a))
        end
        if me.active then
            visibility(me, owner, true)
        end
    end

    -- `sub_80E0B08`: the look; LifeAur's aura hides part 6 but for the local MegaMan; the rising sound; the first
    -- update at once.
    local function init(me: Object)
        local b = (me.state :: State).barrier
        local look = b.look
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = look.anim, look.anim
        sprite:set_animation(look.anim)
        sprite:step()
        sprite.palette = look.palette
        local hidden = look.hidden_parts
        if b.hidden_unless_local_megaman then
            local owner = owner_of(me)
            local local_megaman = owner.actor_type == "player" and owner.ai_index == 0 and owner.alliance == battle.local_side()
            if not local_megaman then
                hidden = PART_6
            end
        end
        sprite.hidden_parts = hidden
        -- (sub_8002EAC: its parts keep their own facing; drawn only.)
        battle.play_sound(b.sound)
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { barrier = "record:barrier", shown = "bool", pop_hit_mod = "u8" },
        -- `sub_80E0AD4`: by state, then its sprite (but while the fight is on and paused).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                -- sub_8016C9C
                me:free()
            end
            if not me.active then
                return
            end
            if not (battle.fighting() and battle.paused()) then
                me:update_sprite_even_paused()
            end
        end,
    }

    -- `sub_80E0D98`: `b`'s visual on `navi`, which links it; it runs while paused too. (Spawned at the caller's
    -- registers; its init places it.)
    local function spawn(navi: Object, b: Barrier): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if not o then
            return nil
        end
        o.alliance, o.flip = navi.alliance, navi.flip
        o.related1 = navi
        navi.barrier_visual = o
        o.run_while_paused = true
        local s = o.state :: State
        s.barrier, s.shown = b, true
        return o
    end

    return { kind = kind, spawn = spawn }
end

return visual
