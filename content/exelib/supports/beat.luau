--!strict
-- Beat, as EXE5 and EXE6 share him (actor object #0x4C in both: EXE6's `sub_80C34E0`, EXE5's 0x080C24C8, the same code),
-- of a game's `BeatLook` (`beat.make`): the NaviCust support that snatches a Mega or Giga chip: the opponent used one,
-- the rules turned the use back (EXE6's `sub_80106C0`, EXE5's 0x0800E418) and summoned him through the supports'
-- controller (./controller). He swoops in from 240 pixels behind the opponent's navi and 112 above it, bounces off
-- it, takes the chip (the opponent's hand moves on) and, a little over a second later, flies off. Then the controller
-- moves on. See docs/engine/chips.md §2.10.

local beat = {}

-- Where he starts: 240 pixels from the opponent's navi toward its own
-- side's back and 112 above it; how fast he swoops (16.16 a tick).
local START_DX = 0xF0
local START_DZ = 0x70_0000
local SPEED_X = 0xC_0000
local SPEED_Z = 0x4_0000
-- His bounce: the X velocity loses this a tick (`byte_80C3628`).
local BOUNCE_BRAKE = 0x4_0000

-- `object_getAllianceDirection`: +1 for the left side, -1 for the right.
local function direction(side: number): number
    return 1 - 2 * side
end

local function glide(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
end

-- `sub_80C36AE` (EXE5's 0x080C268E): behind and above the opponent's
-- navi, swooping toward it.
local function place(me: Object)
    local other = bit32.bxor(me.alliance, 1)
    local target = battle.player(other)
    if not target then
        -- The game reads the missing navi's position through a null
        -- pointer.
        error("Beat has no opponent navi to swoop at (sub_80C36AE reads a null object)")
    end
    local p = target.pos
    me.pos = Vec3.new(int.i32(p.x + direction(other) * START_DX * 0x10000), p.y, int.i32(p.z + START_DZ))
    me.vel = Vec3.new(direction(me.alliance) * SPEED_X, 0, -SPEED_Z)
end

-- A game's Beat, of `look`.
function beat.make(look: BeatLook): SupportActor
    -- `sub_80C3558` (EXE5's 0x080C2538), by phase.
    local function tick(me: Object)
        local p = me.phase
        if p == 0 then
            -- sub_80C3584
            me.timer = 0x14
            battle.play_sound(look.swoop_sound)
            me.phase, me.phase_init = 4, 0
        elseif p == 4 then
            -- sub_80C3598: the swoop; then the bounce.
            glide(me)
            me.timer -= 1
            if me.timer ~= 0 then
                return
            end
            me.anim = 1
            me.timer = 6
            local v = me.vel
            me.vel = Vec3.new(v.x, v.y, int.i32(-v.z))
            me.phase, me.phase_init = 8, 0
        elseif p == 8 then
            -- sub_80C35CE: the bounce; then the snatch.
            glide(me)
            local v = me.vel
            me.vel = Vec3.new(int.i32(v.x - direction(me.alliance) * BOUNCE_BRAKE), v.y, v.z)
            me.timer -= 1
            if me.timer ~= 0 then
                return
            end
            me.anim = 2
            local target = battle.player(bit32.bxor(me.alliance, 1))
            if target then
                battle.advance_hand(target.alliance)
            end
            battle.play_sound(look.snatch_sound)
            me.phase, me.phase_init = 0xC, 0
        elseif p == 0xC then
            -- sub_80C3630
            if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
                return
            end
            me.timer = 0x4B
            me.anim = 3
            me.phase, me.phase_init = 0x10, 0
        elseif p == 0x10 then
            -- sub_80C364A: off he flies.
            me.timer -= 1
            if me.timer ~= 0 then
                return
            end
            me.timer = 0x28
            me.anim = 4
            me.vel = Vec3.new(direction(me.alliance) * SPEED_X, me.vel.y, SPEED_Z)
            me.phase, me.phase_init = 0x14, 0
        elseif p == 0x14 then
            -- sub_80C3678
            glide(me)
            me.timer -= 1
            if me.timer ~= 0 then
                return
            end
            me:set_lifecycle("destroy")
        else
            error(string.format("Beat's phase %#x reads past its table (off_80C356C)", p))
        end
    end

    -- `sub_80C3504` (EXE5's 0x080C24EC, without EXE6's `sprite_decompress`):
    -- his look and his start; his first update at once. (He also sets his
    -- first extra variable to 1, which only the uncalled `sub_80C3700`
    -- reads.)
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.visible = true
        place(me)
        me:set_lifecycle("update")
        tick(me)
    end

    -- `sub_80C34E0`: by state, then his sprite (a freed slot's too, which
    -- does nothing).
    local function update(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- sub_80C36A0: the controller's flag cleared, gone.
            local controller = me.related1 :: Object
            local c = controller.state :: { out: boolean }
            c.out = false
            me:free()
        end
        me:update_sprite()
    end

    local kind = new.kind {
        id = look.id,
        pool = "actor",
        update = update,
    }

    -- `sub_80C3710` (EXE5's 0x080C26F0): Beat, for `controller` (whose `out`
    -- flag he sets, and clears when he goes).
    local function spawn(controller: Object): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if not o then
            return nil
        end
        o.alliance = controller.alliance
        o.related1 = controller
        local c = controller.state :: { out: boolean }
        c.out = true
        o.run_while_dimmed = true
        return o
    end

    return { kind = kind, spawn = spawn }
end

return beat
