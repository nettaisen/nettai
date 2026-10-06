--!strict
-- A magnet's pull (EXE5's attack object #0x66, 0x080D2BE8; EXE5's own): an unseen collision (collision row 0x36, the
-- pulling kind, hitting nothing itself) on the panel 20 pixels ahead of a sliding magnet (./magnet), following it
-- until it crashes (its action 8) or the battle ends. The magnet's (its variant 1) runs while paused and while
-- dimmed. Its variant 0 follows a navi, 40 pixels ahead, while the navi's action is 0xB and its attack's step isn't
-- 0x18: EXE5's MagnetMan as an enemy's attack. MagnetMan the navi's own chip spawns one too
-- (navis/magnetman/chip), and since a player runs that chip as another action, his pull sits on its panel for
-- the tick it appears and goes on the next.

local collision = require("../../rules/collision")

export type State = {
    -- Which way it sits from the magnet (+0x40: 1 or -1).
    dir: number,
    -- Its first parameter: 1 a magnet's, 0 a navi's.
    variant: number,
}

local pull = {}

-- How far ahead of the magnet it sits, and of a navi (pixels).
local AHEAD, NAVI_AHEAD = 0x14, 0x28
-- The action of a navi no player controls that a navi's pull follows it through, and the attack step that ends it.
local NAVI_ACTION, NAVI_DONE = 0x0B, 0x18

local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D2C3A: the hits against it resolve; at the battle's end it goes; outside dimming (or running while dimmed)
-- it goes once the magnet crashes, else keeps 20 pixels ahead of it, at its height.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        finish(me)
        return
    end
    if me.run_while_dimmed or not battle.dimmed() then
        local s = me.state :: State
        local magnet = me.related2 :: Object
        if s.variant == 0 then
            -- 0x080D2C60: a navi's: gone once its attack's step reads 0x18 or its action isn't 0xB. (A player's
            -- action byte is its own action's number, 8 for MagnetMan's NSTackle: never 0xB.)
            if magnet.step == NAVI_DONE or magnet.actor_type == "player" or magnet.action ~= NAVI_ACTION then
                finish(me)
                return
            end
        elseif magnet.action == 8 then
            finish(me)
            return
        end
        local leader = me.related1 :: Object
        local p, lp = me.pos, leader.pos
        local y = bit32.bor(bit32.band(lp.y, 0xFFFF0000), bit32.band(p.y, 0xFFFF))
        local ahead = if s.variant == 0 then NAVI_AHEAD else AHEAD
        local hi = int.u16(s.dir * ahead + bit32.rshift(lp.x, 16))
        local x = bit32.bor(bit32.lshift(hi, 16), bit32.band(p.x, 0xFFFF))
        me.pos = Vec3.new(int.i32(x), int.i32(y), p.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
    end
    c:present()
end

-- 0x080D2C0C: on its panel's center; its collision (row 0x36, hitting nothing, hit modifier 0, no spark).
local function init(me: Object)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.collision_36, collision.nothing, 0)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    me:set_lifecycle("update")
end

pull.kind = new.kind {
    id = "magnetmn/pull",
    pool = "attack",
    state = { dir = "i8", variant = "u8" },
    -- 0x080D2BE8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080D2CBA with variant 1: a pull of `magnet`'s side on (x, y) with `element`, `dir` of it, running while paused
-- and dimmed. (Its position is the spawner's registers, which its init overwrites.)
function pull.spawn(magnet: Object, x: number, y: number, element: number, dir: number): Object?
    local o = battle.spawn(pull.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.dir, s.variant = dir, 1
        o.panel_x, o.panel_y = int.u8(x), y
        o.element = element
        o.alliance = magnet.alliance
        o.related1, o.related2 = magnet, magnet
        o.run_while_paused = true
        o.run_while_dimmed = true
    end
    return o
end

-- 0x080D2CBA with variant 0: a pull of `navi`'s side on (x, y) with `element`, `dir` of the navi, which it follows.
function pull.spawn_for_navi(navi: Object, x: number, y: number, element: number, dir: number): Object?
    local o = battle.spawn(pull.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.dir, s.variant = dir, 0
        o.panel_x, o.panel_y = int.u8(x), y
        o.element = element
        o.alliance = navi.alliance
        o.related1, o.related2 = navi, navi
    end
    return o
end

return pull
