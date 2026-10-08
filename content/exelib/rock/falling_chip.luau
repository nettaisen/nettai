--!strict
-- A chip of broken rock, as EXE6 and EXE4 share it (EXE6's effect object #9, `sub_80E0FA0`; EXE4's #9, 0x080E2914,
-- the same code but for its sprite): spawned with a sideways and an upward speed (EXE6's `sub_80E1084`, EXE4's
-- 0x080E29F8), it flies and falls; below the ground it blinks for 10 ticks and is gone. A falling rock
-- (@exelib/rock/falling) breaks into two, as EXE6's GroundMan's rocks do.
--
-- Each game makes its kind (`falling_chip.make_kind`) with its sprite (its animation 1: the falling rock's sheet).

local falling_chip = {}

export type State = {
    -- ObjectData+0x0C: the sprite's priority.
    priority: number,
}

-- A game's rock chips: the kind, and a chip at `pos` with speeds `vx` and `vz` (16.16), drawn at priority
-- `priority`. (The game also gives it a Param1, which it never reads.)
export type Chips = {
    kind: Kind,
    spawn: (pos: Vec3, vx: number, vz: number, priority: number) -> Object?,
}

-- The fall's acceleration (16.16, down).
local GRAVITY = 0x3000

function falling_chip.make_kind(id: string, sprite: SpriteAsset): Chips
    -- EXE6's `sub_80E0FC0`, EXE4's 0x080E2934: visible, its sprite in animation 1, 10 ticks to blink.
    local function init(me: Object, s: State)
        me.visible = true
        local sp = me.sprite
        sp:load(sprite)
        sp.shadow = "ground"
        me.anim = 1
        sp:set_animation(1)
        sp:step()
        sp.priority = s.priority
        me.timer = 10
        sp:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_lifecycle("update")
    end

    -- EXE6's `sub_80E1028`, EXE4's 0x080E299C: fly and fall; under the ground, on to blinking.
    local function fly(me: Object)
        local v = me.vel
        local vz = v.z - GRAVITY
        me.vel = Vec3.new(v.x, v.y, vz)
        local p = me.pos
        local z = p.z + vz
        me.pos = Vec3.new(p.x + v.x, p.y, z)
        if z < 0 then
            me.action = 4
            me.phase = 0
            me.phase_init = 0
        end
    end

    -- EXE6's `sub_80E1054`, EXE4's 0x080E29C8: blink (shown on the ticks the timer's bit 1 is set), then gone.
    local function blink(me: Object): boolean
        me.visible = bit32.btest(me.timer, 2)
        me.timer -= 1
        if me.timer == 0 then
            me:free()
            return false
        end
        return true
    end

    -- EXE6's `sub_80E100A`, EXE4's 0x080E297E.
    local function tick(me: Object)
        if me.action == 0 then
            fly(me)
        elseif me.action == 4 then
            if not blink(me) then
                return
            end
        else
            error(string.format("%s: action %d runs off its table (sub_80E100A)", id, me.action))
        end
        me:update_sprite()
    end

    local kind = new.kind {
        id = id,
        pool = "effect",
        state = { priority = "u8" },
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, me.state :: State)
                tick(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
        end,
    }

    local function spawn(pos: Vec3, vx: number, vz: number, priority: number): Object?
        local o = battle.spawn(kind, pos)
        if o then
            (o.state :: State).priority = priority
            o.vel = Vec3.new(vx, 0, vz)
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return falling_chip
