--!strict
-- A snake (the original's attack object #0x73, `sub_80D2EBC`;
-- docs/engine/dimming-chip-effects.md §13): the shared snake
-- (@exelib/snake/snake) with BN6's look. It rises out of its hole, waits,
-- then leaps at its target panel and strikes it, or flies on off the field
-- when it has none. Snake's nest (./nest) sends them out and counts them.

local collision = require("../../rules/collision")
local common = require("@exelib/snake/snake")
local trajectory = require("../../lib/trajectory")

local snake = {}

-- The nest's count of the snakes out (the original's pointer to it).
local function counter_add(me: Object, n: number)
    local nest = (me.state :: SnakeState).nest
    if nest and nest.active then
        local s = nest.state :: { snakes: number }
        s.snakes += n
    end
end

snake.kind = common.make_kind("bn6:snake/snake", {
    trajectory = trajectory,
    sprite = asset.sprite("bn6:snake"),
    rise_sound = asset.sound("bn6:appear"),
    leap_sound = asset.sound("bn6:aqua-needle-2"),
    -- 20 ticks.
    wait = 0x14,
    -- Its strike (`sub_80C53A6`, 0x04050501): an attack hitting navis with
    -- hit modifier 3.
    strike_element = 0,
    strike_spark = define.spark { sprite = asset.sprite("bn6:pink-flash"), anim = 0, palette = 0 },
    strike_hit_mod = 3,
    is = collision.attack,
    hits = collision.hits_navis,
    joined = function(me: Object)
        counter_add(me, 1)
    end,
    left = function(me: Object)
        counter_add(me, -1)
    end,
})

-- `sub_80D30A4`: a snake of `nest`'s side and facing in the hole (`x`,
-- `y`), leaping at (`tx`, `ty`) (`tx` 0xFF: none), with `nest`'s damage
-- word.
function snake.spawn(nest: Object, x: number, y: number, tx: number, ty: number): Object?
    return common.spawn(snake.kind, nest, x, y, tx, ty, 0)
end

return snake
