--!strict
-- GutsMan's quake (attack object #0x1C, 0x080D055C: EXE4's own), which his hammer sets off on the panel ahead of him:
-- unseen there, on a solid panel, it shakes the field for 40 ticks with a crash and cracks the solid panels from its
-- column to the field's far end, six of them at random (GutsMan's) or all (GutsMnSP's and GutsMnDS's); 30 ticks
-- later rocks fall (../../objects/falling_rock) on two of the other side's free solid panels at random, and on the
-- panel its navi stands on.
--
-- (Action 0x44, a navi's hammer that no netbattle starts, spawns it too, with a Param1 of its own: given 0, it cracks
-- Param2 of the other side's panels at random and drops its rocks around the other side's navi or the target
-- 0x0800C456 picks, by its owner's flags 0xA000. The game tests Param1 against its routine's address left in r1,
-- 0x080D058F: GutsMan's 1 and 2 have a bit of it.)

local falling_rock = require("../../objects/falling_rock")
local panels = require("@exelib/panels")

local quake = {}

type State = {
    -- It cracks all the solid panels ahead, not six at random (Param1 2, not 1).
    cracks_all: boolean,
}

local SOUND = asset.sound("metal-punch")
local SOLID = 0x10
-- Its rocks fall on solid panels without these (bodies of the stage's and the sides' objects: 0x080D07FA).
local ROCK_FORBID = 0x0380_0000
-- The other side's navi's body, by side (0x080D088C).
local OTHER_NAVI: { [number]: number } = { [0] = 0x20_0000, [1] = 0x40_0000 }
-- How many panels it cracks at random, and rocks it drops at random.
local CRACKS, ROCKS = 6, 2

local ROCK = falling_rock.make("gutsman/rock")

-- `object_isPanelSolid`, which reads no panel off the field (0x0800A3D8).
local function solid(x: number, y: number): boolean
    return field.valid(x, y) and field.solid(x, y)
end

-- 0x080D0654: the solid panels from its column on toward the other side, rows 3 to 1 of each column, while the
-- column is -1 to 6; all of them cracked (last listed first), or six after a shuffle (the sixth first). (With
-- fewer than six the game reads stale stack bytes as the rest; the port cracks only those it listed.)
local function crack(me: Object, all: boolean)
    local dir = panels.side_direction(me.alliance)
    local list: { number } = {}
    local x = me.panel_x
    while true do
        for y = 3, 1, -1 do
            if solid(x, y) then
                list[#list + 1] = panels.pack(x, y)
            end
        end
        x += dir
        if x < -1 or x > 6 then
            break
        end
    end
    local n = #list
    if n == 0 then
        return
    end
    local count = n
    if not all then
        panels.shuffle(list, n, n)
        count = CRACKS
    end
    for i = count, 1, -1 do
        local p = list[i]
        if p then
            field.crack(panels.x(p), panels.y(p))
        end
    end
end

-- A rock of its element and damage word on (x, y), running while dimmed.
local function rock(me: Object, x: number, y: number)
    local o = ROCK.spawn(me, x, y, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    if o then
        o.run_while_dimmed = true
    end
end

-- 0x080D07C4: from the far column of the field (the other side's back: 6, or 1) back to its own, rows 3 to 1, the
-- solid panels nothing else stands on: the first the other side's navi stands on is its target, the rest listed;
-- rocks on two of the listed after a shuffle (the second first), then on the target.
local function drop(me: Object)
    local mask = OTHER_NAVI[me.alliance]
    local back = -panels.side_direction(me.alliance)
    local stop = me.panel_x + back
    local x = 6 - 5 * me.alliance
    local list: { number } = {}
    local tx, ty = 0, 0
    repeat
        for y = 3, 1, -1 do
            if field.check(x, y, SOLID, ROCK_FORBID) then
                if tx == 0 and bit32.btest(field.flags(x, y), mask) then
                    tx, ty = x, y
                else
                    list[#list + 1] = panels.pack(x, y)
                end
            end
        end
        x += back
        if x < -1 or x > 8 then
            error("GutsMan's quake looks for its rocks' panels past its own column (0x080D0836 would loop on)")
        end
    until x == stop
    local n = #list
    if n > 0 then
        panels.shuffle(list, n, n)
        for i = math.min(n, ROCKS), 1, -1 do
            rock(me, panels.x(list[i]), panels.y(list[i]))
        end
    end
    if tx ~= 0 then
        rock(me, tx, ty)
    end
end

-- 0x080D058E: the shake and its sound and the cracks; 30 ticks; then the rocks, and it goes.
local function tick(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND)
        battle.shake_camera(1, 40)
        crack(me, (me.state :: State).cracks_all)
        me.timer = 30
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    drop(me)
    me:set_lifecycle("destroy")
end

quake.kind = new.kind {
    id = "gutsman/quake",
    pool = "attack",
    state = { cracks_all = "bool" },
    -- 0x080D055C: by its state.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080D057C: on a solid panel, on; else gone.
            me:set_lifecycle(if field.solid(me.panel_x, me.panel_y) then "update" else "destroy")
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- 0x080D0894: a quake on (x, y) of GutsMan's side, his, with his element and damage word, running while dimmed (his
-- hammer's: 0x080B9AC8). (Its position is the spawner's registers: the row, the element and what the caller left in
-- r3; it never sets one.)
function quake.spawn(gutsman: Object, x: number, y: number, cracks_all: boolean): Object?
    local o = battle.spawn(quake.kind, Vec3.new(y, gutsman.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = gutsman.element
    o.related1 = gutsman
    o:set_damage_word(bit32.bor(gutsman.damage, bit32.lshift(gutsman.stamina, 16)))
    o.alliance = gutsman.alliance
    ;(o.state :: State).cracks_all = cracks_all
    o.run_while_dimmed = true
    return o
end

return quake
