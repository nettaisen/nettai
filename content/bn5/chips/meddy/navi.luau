--!strict
-- Meddy, the navi of BN5's Meddy chips (BN5's actor object #0x12, 0x080BBAA0; BN5's own, built on BN6's navi
-- template), brought by their navi chip controller. She appears, a pixel up and nearer (and goes at once off a
-- panel that isn't solid), stands 40 ticks and winds up (animation 6): 8 ticks in, with a sound, she throws two
-- capsules (chips/meddy/capsule, variant 0, running while dimmed), each at a panel beside an enemy navi (the first
-- such panel of four, solid and free if any is, in the enemy's area; the line it bursts along by the side it lies
-- on) or else at a panel of the enemy's area, the second never at the first's panel; 141 ticks in she leaves, and
-- the controller moves on.

local capsule = require("./capsule")
local navi_chips = require("@exelib/navi-chips/navi")
local panels = require("@exelib/panels")

export type State = {
    -- The controller waiting for her (the flag it gave her, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 4 for DS), which picks her palette.
    level: number,
    -- The enemy navi's panel her last capsule went beside (+0x60, +0x64), and her last capsule's panel (+0x68,
    -- +0x6C), which the next pick reserves.
    enemy_x: number,
    enemy_y: number,
    pick_x: number,
    pick_y: number,
}

local meddy = {}

local SPRITE = asset.sprite("meddy")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_THROW = asset.sound("minibomb-throw")
-- By the level (0x0800DA8A: navi 6's count, 2, by 0x0800DAE8's row): DS's 6 (SP's level is 0).
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 4, 6 }
local SOLID = 0x10
-- When she throws (ticks left of her windup).
local THROW = 0x84

type Filter = { require: number, forbid: number }
-- By side: an enemy navi in the enemy's area (0x080BBE74); a free solid panel of the enemy's area, then any free
-- one of it (0x080BBE88, 0x080BBEBC); a panel of the enemy's area (0x080BBE9C).
local ENEMY_NAVI: { [number]: Filter } = {
    [0] = { require = 0x0400_0020, forbid = 0 },
    [1] = { require = 0x0800_0000, forbid = 0x20 },
}
local BESIDE: { [number]: Filter } = {
    [0] = { require = 0x30, forbid = 0x0F88_0080 },
    [1] = { require = 0x10, forbid = 0x0F88_00A0 },
}
local BESIDE_ANY: { [number]: Filter } = {
    [0] = { require = 0x20, forbid = 0x0F88_0080 },
    [1] = { require = 0, forbid = 0x0F88_00A0 },
}
local ENEMY_AREA: { [number]: Filter } = {
    [0] = { require = 0x20, forbid = 0 },
    [1] = { require = 0, forbid = 0x20 },
}
-- The panels beside a navi, in order (0x080BBEB0): behind and ahead (a capsule there bursts along its row), above
-- and below (along its column).
local AROUND = { { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } }

-- The panels around (x, y) that meet `f`, in order.
local function beside(x: number, y: number, f: Filter): { number }
    local list: { number } = {}
    for _, o in AROUND do
        local px, py = x + o[1], y + o[2]
        if field.check(int.u8(px), int.u8(py), f.require, f.forbid) then
            list[#list + 1] = panels.pack(px, py)
        end
    end
    return list
end

-- 0x080BBCAA: her next capsule's panel (column 0: none) and its line. Her last capsule's panel is reserved while
-- she picks: the first enemy navi in the enemy's area (shuffled, not the last one's unless alone), the first panel
-- beside it (BN5 draws a number and takes the first anyway); with none, a panel of the enemy's area (shuffled),
-- not her last capsule's.
local function pick(me: Object): (number, number, number)
    local s = me.state :: State
    me:reserve_panel(s.pick_x, s.pick_y)
    local x, y, line = 0, 0, 0
    local found = false
    local enemies = panels.filtered_except(me, ENEMY_NAVI[me.alliance].require, ENEMY_NAVI[me.alliance].forbid)
    local ex: number?, ey: number? = nil, nil
    if #enemies == 1 then
        ex, ey = panels.x(enemies[1]), panels.y(enemies[1])
    elseif #enemies > 1 then
        panels.shuffle(enemies, #enemies, #enemies)
        for _, p in enemies do
            if panels.x(p) ~= s.enemy_x or panels.y(p) ~= s.enemy_y then
                ex, ey = panels.x(p), panels.y(p)
                break
            end
        end
    end
    if ex and ey then
        local list = beside(ex, ey, BESIDE[me.alliance])
        if #list == 0 then
            list = beside(ex, ey, BESIDE_ANY[me.alliance])
        end
        if #list ~= 0 then
            battle.rng_positive()
            x, y = panels.x(list[1]), panels.y(list[1])
            found = true
            line = -1
            for i, o in AROUND do
                if x == ex + o[1] and y == ey + o[2] then
                    line = if i <= 2 then 1 else 0
                    break
                end
            end
            if line < 0 then
                battle.rng_positive()
                line = 0
            end
        end
    end
    if not found then
        local area = panels.filtered_except(me, ENEMY_AREA[me.alliance].require, ENEMY_AREA[me.alliance].forbid)
        if #area == 0 then
            error("Meddy's pick (0x080BBCAA) finds no panel of the enemy's area and stores stale stack")
        end
        panels.shuffle(area, #area, #area)
        for _, p in area do
            y = panels.y(p)
            if panels.x(p) ~= s.pick_x or y ~= s.pick_y then
                x = panels.x(p)
                battle.rng_positive()
                line = 0
                break
            end
        end
    end
    me:unreserve_panel(s.pick_x, s.pick_y)
    s.pick_x, s.pick_y = x, y
    -- (Without an enemy navi this pick, the game stores what its stack held; that is read again only when there are
    -- two enemy navis.)
    if ex and ey then
        s.enemy_x, s.enemy_y = ex, ey
    end
    return x, y, line
end

local function throw(me: Object)
    local x, y, line = pick(me)
    if x ~= 0 then
        capsule.throw(me, me.element, {
            variant = 0,
            line = line,
            target_x = x,
            target_y = y,
            damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
        })
    end
end

-- Action 0 (0x080BBB3C): she appears (animation 3, a sound); 4 ticks; then she stays, or goes off a panel that isn't
-- solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if navi_chips.wait_on_init(me, 4, true) then
        me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0xC)
    end
end

-- Action 4 (0x080BBB8C): 40 ticks standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if navi_chips.wait_on_init(me, 0x28, true) then
        me:set_action(8)
    end
end

-- Action 8 (0x080BBBB4): her windup (animation 6), 141 ticks; 8 in, with a sound, the two capsules.
local function windup(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 6
        me.timer = 0x8C
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left == THROW then
        battle.play_sound(SOUND_THROW)
        local s = me.state :: State
        s.enemy_x, s.enemy_y, s.pick_x, s.pick_y = 0, 0, 0, 0
        throw(me)
        throw(me)
    elseif left < 0 then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BBC2C): she leaves (animation 4), 4 ticks; then the controller moves on, and she goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if navi_chips.wait_on_init(me, 4, true) then
        navi_chips.leave(me, (me.state :: State).controller)
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = windup, [0xC] = leave }

-- 0x080BBAC4: on her panel, a pixel up and nearer; her sprite (animation 0, her shadow), her palette by the level,
-- facing her side's way.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, int.i32(p.y + 0x1_0000), -0x1_0000)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = PALETTES[(me.state :: State).level] or 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

meddy.kind = define.kind {
    id = "meddy/navi",
    pool = "actor",
    state = {
        controller = "object",
        level = "u8",
        enemy_x = "u8",
        enemy_y = "u8",
        pick_x = "u8",
        pick_y = "u8",
    },
    -- 0x080BBAA0.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("Meddy action %#x reads past her table (0x080BBB2C)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BBC5C: Meddy for `user`, reporting to `controller` (the summon table's entry 18), of a chip of `level`.
function meddy.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(meddy.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- Meddy's and MeddySP's (their Param1s 0), MeddyDS's (4).
meddy.summon = meddy.summon_at(0)
meddy.summon_sp = meddy.summon_at(0)
meddy.summon_dark = meddy.summon_at(4)

return meddy
