--!strict
-- GroundCross's A-charge, routine 0x18 (`sub_8012CB2`). `off_80117D4[0x18]`
-- is a nullsub: the chip-use framework (`sub_800FB54`) calls this routine
-- instead before a charged chip is used, and what it leaves in r0 is the
-- chip use's charge argument (`sub_80127C0`). So `setup` returns that
-- argument, not an action: with an enemy on the field, rocks fall on three
-- enemy panels (objects/falling-rock) and it is 2; without one, 0.

local weapon = require("../../../../lib/weapon")
local falling_rock = require("../../../../objects/falling-rock/falling_rock")

local ground_charge = {}

-- The enemies it counts: NameIDs 0..0xBA (viruses) and 0x100..0x1C3.
local VIRUSES, NAVIS = { 0, 0xBA }, { 0x100, 0x1C3 }

-- The rocks: Break, counter byte 0x8A, 30 damage and 20 more per point of
-- buster damage (up to 5).
local BREAK, HIT_PARAM = 0x10, 0x8A

-- Where a duplicate target may move to (`byte_8012DD4`, by side): panels
-- with every `require` flag and no `forbid` flag.
local SPOTS = { [0] = { require = 0x20, forbid = 0x80 }, [1] = { require = 0x00, forbid = 0xA0 } }

-- A panel as the game packs it: x | y << 4.
local function pack(x: number, y: number): number
    return bit32.bor(x, bit32.lshift(y, 4))
end

-- `object_getEnemyByNameRange`: the side's alive actors with a NameID in
-- the range, in slot order.
local function enemies_named(side: number, range: { number }, into: { Object })
    for _, o in battle.alive_actors(side) do
        local id = o.name_id
        if id >= range[1] and id <= range[2] then
            table.insert(into, o)
        end
    end
end

-- `sub_8015C94`: a random panel of `navi`'s choosing (one draw), not its
-- own; its own panel when there is none.
local function random_panel(navi: Object, require: number, forbid: number): (number, number)
    local found = {}
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            if field.check(x, y, require, forbid) and not (x == navi.panel_x and y == navi.panel_y) then
                table.insert(found, pack(x, y))
            end
        end
    end
    if #found == 0 then
        return navi.panel_x, navi.panel_y
    end
    local p = found[int.tmod(battle.rng_positive(), #found) + 1]
    return bit32.band(p, 7), bit32.rshift(p, 4)
end

-- `sub_8012D8C`: if targets i and j are the same panel, j moves elsewhere
-- (`sub_8012DB8`), with i reserved meanwhile so it isn't picked.
local function separate(navi: Object, t: { number }, i: number, j: number)
    if t[i] ~= t[j] then
        return
    end
    local ix, iy = bit32.band(t[i], 7), bit32.rshift(t[i], 4)
    navi:reserve_panel(ix, iy)
    local spot = SPOTS[navi.alliance]
    local x, y = random_panel(navi, spot.require, spot.forbid)
    -- (A picked panel at x 0 would read as none: the game keeps 0.)
    t[j] = if x ~= 0 then pack(x, y) else 0
    navi:unreserve_panel(ix, iy)
end

-- `sub_8012CB2`.
function ground_charge.setup(navi: Object): number
    local enemy = bit32.bxor(navi.alliance, 1)
    local found: { Object } = {}
    enemies_named(enemy, VIRUSES, found)
    enemies_named(enemy, NAVIS, found)
    local n = #found
    if n == 0 then
        return 0
    end
    -- `sub_8012D24`: the enemies' panels (three targets from fewer by
    -- repeating the last), then the repeats moved elsewhere
    -- (`sub_8012D70`). With four enemies the game repeats the fourth past
    -- the first three and separates those copies, while the rocks fall
    -- on the first three.
    local t = { 0, 0, 0, 0, 0, 0 }
    for k, o in found do
        t[k] = pack(o.panel_x, o.panel_y)
    end
    local base = 0
    if n == 1 then
        t[2], t[3] = t[1], t[1]
    elseif n == 2 then
        t[3] = t[2]
    elseif n == 4 then
        base = 3
        t[5], t[6] = t[4], t[4]
    end
    separate(navi, t, base + 1, base + 2)
    separate(navi, t, base + 1, base + 3)
    separate(navi, t, base + 2, base + 3)
    local damage = bit32.bor(bit32.lshift(HIT_PARAM, 16), weapon.charge_damage(navi, 0x1E, 0x14))
    for k = 1, 3 do
        falling_rock.spawn(navi, bit32.band(t[k], 7), bit32.rshift(t[k], 4), BREAK, damage)
    end
    -- (And a camera shake: presentation only.)
    return 2
end

return ground_charge
