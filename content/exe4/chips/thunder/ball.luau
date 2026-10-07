--!strict
-- EXE4's thunder ball (attack object 0x2A, 0x080D2000; spawned at 0x080D2156): the shared ball (@exelib/thunder/ball,
-- EXE6's code) with EXE4's look: one sprite (10-11, animation 0; EXE4's ball is never big, nor fast: its aim,
-- 0x080D2176, has one speed), an attack hitting navis (hit modifier 0, the elec spark), its sound (0xAD), and its own
-- search for its target (0x080D21D4: from its side's back column toward the other side, not from where it is).
-- Thunder1-3's, whose shot (@exelib/thunder/shoot, action 0x26) raises no arm: `thunder.shoot { ... }`.

local ball = require("@exelib/thunder/ball")
local collision = require("../../rules/collision")
local shoot = require("@exelib/thunder/shoot")
local sparks = require("../../lib/sparks")

local thunder = {}

-- The other side's navi on a panel, by the ball's side (0x080D221C).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }

-- 0x080D21D4: the first column holding the other side's navi, bottom row up, from the ball's side's back column
-- toward the other side (side 0's from column 0 to 6, side 1's from 6 to 1); none, (0, 0).
local function find_target(me: Object): (number, number)
    local bodies = ENEMY_BODY[me.alliance]
    local first, last, step = 0, 6, 1
    if me.alliance ~= 0 then
        first, last, step = 6, 1, -1
    end
    for x = first, last, step do
        for y = 3, 1, -1 do
            if bit32.btest(field.flags(x, y), bodies) then
                return x, y
            end
        end
    end
    return 0, 0
end

thunder.kind = ball.make_kind("thunder/ball", {
    sprite = asset.sprite("thunder-ball"),
    small_anim = 0,
    is = collision.attack,
    target = collision.hits_navis,
    hit_mod = 0,
    spark = sparks.elec,
    sound = asset.sound("thunder-ball"),
    find_target = find_target,
})

-- The shot: no arm raised, the navi in its animation 0x12 (0x080ECE3C).
local LOOK: ThunderShotLook = { ball = thunder.kind, anim = 0x12 }

-- A chip's shot.
function thunder.shoot(spec: ThunderShot): Action
    return shoot.action(spec, LOOK)
end

return thunder
