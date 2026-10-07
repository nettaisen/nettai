--!strict
-- EXE4's barrier visual (effect object 7, 0x080E2418; EXE6's #7, `sub_80E0AD4`, with EXE4's differences): it sits on
-- its navi and follows the navi's barrier (the collision data's): up, it shows; worn down, it goes down (and comes
-- back up when the barrier has HP again); with the barrier gone, it goes. The navi links it (`barrier_visual`: EXE4's
-- object +0x50, which 0x08012DEC clears with the barrier), and hides and shows it with itself (its `shown`, +0x0C).
-- Unlike EXE6's: no blown action (wind takes EXE4's barrier away at once), no sound going down, its sprite updated
-- paused or not (0x080144BC), running while paused only until the fight starts, and placed by a table of offsets by
-- its navi's number (NaviStats +0x23: 0x080E2572, or 0x080E2558 for an aura, type 6 on), not an attach point:
-- MegaMan's rows are none, and EXE4's content has no other navi.

local visual = {}

export type State = {
    barrier: Barrier,
    -- Shown with its navi (+0x0C).
    shown: boolean,
}

-- Its actions.
local UP, DOWN = 0, 4

-- The navi forgets it, if it is the one the navi links.
local function unlink(me: Object)
    local owner = me.related1
    if owner and owner.barrier_visual == me then
        owner.barrier_visual = nil
    end
end

local function go(me: Object, action: number)
    me:set_action(action)
end

-- 0x080E258C: up while the navi links a visual and its barrier stands with HP; else down.
local function up(me: Object, owner: Object)
    local c = owner.collision
    if owner.barrier_visual == nil or c.barrier == "none" or c.barrier_hp == 0 then
        go(me, DOWN)
    end
end

-- 0x080E25B0: going down (the next animation); back up, with its sound, when the barrier has HP again; hidden at the
-- end of the animation while the barrier stands; gone at it once the barrier is.
local function down(me: Object, owner: Object)
    local b = (me.state :: State).barrier
    if me.phase_init == 0 then
        me.anim = me.anim + 1
        me.phase_init = 4
    end
    local c = owner.collision
    local last_frame = bit32.btest(me.sprite.frame_flags, 0x80)
    if c.barrier ~= "none" then
        if c.barrier_hp ~= 0 then
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

-- 0x080E24A4: seen unless the viewer is blind to its side, shown with its navi; once the fight is on, no longer run
-- while paused; on its navi, 2 pixels up the field and lower; its action.
local function tick(me: Object)
    local owner = me.related1 :: Object
    me.visible = true
    me:hide_from_blind()
    if not (me.state :: State).shown then
        me.visible = false
    end
    if battle.fighting() then
        me.run_while_paused = false
    end
    local p = owner.pos
    me.pos = Vec3.new(p.x, p.y - 0x20000, p.z - 0x20000)
    local a = me.action
    if a == UP then
        up(me, owner)
    elseif a == DOWN then
        down(me, owner)
    else
        error(string.format("a barrier's visual's action %#x reads past 0x080E2550", a))
    end
end

-- 0x080E243C: the look (by the barrier's type, 0x080E23D8), animated a step; the parts its look hides (MegaMan's:
-- 0x080E247A gives another navi 0x02000000); the rising sound (0x125); the first update at once.
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
    sprite.hidden_parts = look.hidden_parts
    -- (sub_8002EAC: its parts keep their own facing; drawn only.)
    battle.play_sound(b.sound)
    me:set_lifecycle("update")
    tick(me)
end

visual.kind = new.kind {
    id = "barrier-visual",
    pool = "effect",
    state = { barrier = "record:barrier", shown = "bool" },
    -- 0x080E2418: by state, then its sprite, paused or not (0x080144BC).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        if me.active then
            me:update_sprite_even_paused()
        end
    end,
}

-- 0x080E2622: `b`'s visual on `navi`, which links it; it runs while paused (until the fight starts) and shows with
-- its navi. (Spawned at the caller's registers; its init places it.)
function visual.spawn(navi: Object, b: Barrier): Object?
    local o = battle.spawn(visual.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.alliance = navi.alliance
    o.related1 = navi
    navi.barrier_visual = o
    o.run_while_paused = true
    local s = o.state :: State
    s.barrier, s.shown = b, true
    return o
end

return visual
