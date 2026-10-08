--!strict
-- Tango, as EXE5 and EXE6 share her (actor object #0x4D in both: EXE6's `sub_80C3734`, EXE5's 0x080C2714, the same code),
-- of a game's `TangoLook` (`tango.make`): the NaviCust support that heals her navi once, in link battles, when its HP
-- drops to a quarter (EXE6's `sub_8010660`, EXE5's 0x0800E3B6): summoned through the supports' controller
-- (./controller), she drops in 33 pixels in front of her navi, lands, sings, and throws a heal (./heal) that arcs onto
-- her navi's panel; once it has landed she waits 10 ticks, bows, and rises away. Then the controller moves on. See
-- docs/engine/chips.md §2.10.

export type State = {
    -- Her heal is out (object +0x0C: the heal clears it when it lands).
    healing: boolean,
}

local tango = {}

-- Where she drops from: 33 pixels in front of her navi, 60 up; how fast
-- she drops and rises (Z, 16.16 a tick), and where she lands.
local FRONT = 0x21_0000
local START_Z = 0x3C_0000
local STEP_Z = 0x4_0000
local LAND_Z = 0x10
-- The heal: from 16 pixels back toward her navi, 10 up.
local HEAL_FRONT = 0x10_0000
local HEAL_Z = 0xA_0000

-- `object_getEnemyDirection`: +1 for the left side, -1 for the right.
local function enemy_direction(me: Object): number
    return 1 - 2 * me.alliance
end

-- `sub_80C390E` (EXE5's 0x080C28E6): in front of her navi, up high (and a
-- pixel down the field, but where the look says not: EXE4's 0x080CAB88).
local function place(me: Object, nudge: boolean)
    local navi = battle.player(me.alliance)
    if not navi then
        -- The game reads the missing navi's panel through a null pointer.
        error("Tango has no navi to heal (sub_80C390E reads a null object)")
    end
    me.panel_x, me.panel_y = navi.panel_x, navi.panel_y
    me:set_coordinates_from_panel()
    local p = me.pos
    local y = if nudge then int.i32(p.y + 0x1_0000) else p.y
    me.pos = Vec3.new(int.i32(p.x + enemy_direction(me) * FRONT), y, START_Z)
end

-- A wait's tick (`ldrh`, `sub`, `strh`, `bge`): false once it counts past
-- zero.
local function waiting(me: Object): boolean
    local t = me.timer - 1
    me.timer = int.u16(t)
    return t >= 0
end

local function next_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- A game's Tango, of `look`.
function tango.make(look: TangoLook): SupportActor
    -- `loc_80C37BC` (EXE5's 0x080C2794), by phase.
    local function tick(me: Object)
        local p = me.phase
        local s = me.state :: State
        if p == 0 then
            -- sub_80C37EC: a sparkle as she appears; 17 ticks.
            if me.phase_init == 0 then
                battle.effect(me.pos, look.arrive)
                me.timer = 0x10
                me.phase_init = 4
            end
            if not waiting(me) then
                next_phase(me, 4)
            end
        elseif p == 4 then
            -- sub_80C3816: she drops, 4 pixels a tick, to 16 up; then lands.
            local at = me.pos
            local z = int.i32(at.z - STEP_Z)
            me.pos = Vec3.new(at.x, at.y, z)
            if int.i32(int.lsr(z, 16)) > LAND_Z then
                return
            end
            me.anim, me.anim_loaded = 1, 0
            me.pos = Vec3.new(at.x, at.y, 0)
            me.timer = 0x1E
            battle.play_sound(look.land_sound)
            next_phase(me, 8)
        elseif p == 8 then
            -- sub_80C383E
            me.timer -= 1
            if me.timer ~= 0 then
                return
            end
            me.anim = 2
            next_phase(me, 0xC)
        elseif p == 0xC then
            -- sub_80C3852: the throw, onto her navi's panel.
            if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
                return
            end
            local navi = battle.player(me.alliance)
            if navi then
                local at = me.pos
                local from = Vec3.new(int.i32(at.x - enemy_direction(me) * HEAL_FRONT), at.y, HEAL_Z)
                look.heal.spawn(me, from, navi.panel_x, navi.panel_y)
            end
            me.timer = 0xA
            battle.play_sound(look.throw_sound)
            next_phase(me, 0x10)
        elseif p == 0x10 then
            -- sub_80C38A0: once the heal has landed, 10 ticks; then her bow.
            if s.healing then
                return
            end
            me.timer -= 1
            if me.timer ~= 0 then
                return
            end
            me.anim = 3
            next_phase(me, 0x14)
        elseif p == 0x14 then
            -- sub_80C38BA
            if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
                return
            end
            me.anim = 0
            me.timer = 0xA
            me.pos = Vec3.new(me.pos.x, me.pos.y, 0x10_0000)
            next_phase(me, 0x18)
        elseif p == 0x18 then
            -- sub_80C38DA: she rises for 9 ticks, and goes in a puff.
            me.timer -= 1
            if me.timer ~= 0 then
                local at = me.pos
                me.pos = Vec3.new(at.x, at.y, int.i32(at.z + STEP_Z))
                return
            end
            battle.effect(me.pos, look.puff)
            me:set_lifecycle("destroy")
        else
            error(string.format("Tango's phase %#x reads past its table (off_80C37D0)", p))
        end
    end

    -- `sub_80C3758` (EXE5's 0x080C2738, without EXE6's `sprite_decompress`):
    -- her look and her place; her first update at once.
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.visible = true
        place(me, look.pixel_nudge ~= false)
        me.timer = 0x28
        battle.play_sound(look.arrive_sound)
        me:set_lifecycle("update")
        tick(me)
    end

    -- `sub_80C3734`: by state, then her sprite (a freed slot's too, which
    -- does nothing).
    local function update(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- sub_80C3900: the controller's flag cleared, gone.
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
        state = { healing = "bool" },
        update = update,
    }

    -- `sub_80C3946` (EXE5's 0x080C291E): Tango, for `controller` (whose
    -- `out` flag she sets, and clears when she goes).
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

    -- Her host stays while she comes (the controller's first phase skips the warp out, its fourth the warp in).
    return { kind = kind, spawn = spawn, warps = false }
end

return tango
