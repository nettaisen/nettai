--!strict
-- SearchMan's scope (EXE5's effect object #0x43, 0x080E5208; EXE6 keeps its code, `sub_80E55C0`, which no EXE6 chip
-- spawns): it starts on the top of the first column past its user's area, and sweeps the columns from there to the
-- far edge, down one column and up the next, a panel every 6 ticks (4 for the SP and DS chips), until its user
-- presses A while the battle is dimmed or 300 ticks pass; then it locks on (animation 1) and gives SearchMan the
-- panel. B pressed with A, or within 5 ticks of it, powers his last shot.

local scope = {}

export type State = {
    -- The chip's level (its Param1): a faster sweep for SP and DS.
    level: number,
    -- Its look: the sight (its +0x64; SearchMan's is 0: the scope).
    sight: boolean,
    -- SearchMan's user (+0x60), whose new presses while dimmed it reads (+0x58: his AIData+0x2C).
    user: Object?,
    -- Its sweep: down (+5), toward the far edge (+7), between the columns it may not reach (+0x14, +0x15).
    down: boolean,
    outward: boolean,
    near: number,
    far: number,
    -- The ticks left for B after A (+6).
    late_b: number,
}

local SIGHT = asset.sprite("sight")
local SCOPE = asset.sprite("search-scope")
-- (0x10E: 0x080E52D4 loads it as 15 + 255.)
local SOUND_MOVE = asset.sound("erase-man")
local SOUND_LOCK = asset.sound("mark")
local SOUND_BONUS = asset.sound("ok-8b")
local TIME_LIMIT = 0x12C

-- `sub_800D5F0`: how many panels of column `x` aren't `side`'s.
local function not_sides(x: number, side: number): number
    local n = 0
    for y = 1, 3 do
        local p = field.panel(x, y)
        if p and p.alliance ~= side then
            n += 1
        end
    end
    return n
end

-- 0x080E5416: from the far edge back, the first column with a panel not its side's; from there on back, the first
-- column wholly its side's. The column past that one, and the first.
local function columns(me: Object): (number, number)
    local back = -me.facing
    local x = 6 - 5 * bit32.bxor(me.alliance, me.flip)
    while not_sides(x, me.alliance) == 0 do
        x += back
        if x - 1 < 0 or x - 1 > 5 then
            break
        end
    end
    local first = x
    while not_sides(x, 1 - me.alliance) ~= 3 do
        x += back
        if x - 1 < 0 or x - 1 > 5 then
            break
        end
    end
    return x - back, first
end

-- 0x080E5228: on the top of the first column past its user's area, its sweep between the column before and the one
-- past the far edge; 300 ticks.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(if s.sight then SIGHT else SCOPE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    -- 0x080E53F4.
    local x, first = columns(me)
    me.panel_x = int.u8(x)
    s.far = int.u8(first + me.facing)
    s.near = int.u8(x - me.facing)
    me.panel_y = 1
    me:set_coordinates_from_panel()
    s.down, s.outward = true, true
    me.timer2 = TIME_LIMIT
    s.late_b = 0
    me.visible = true
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_lifecycle("update")
    me.action = 4
end

-- 0x080E52E6: a row down (or up); false once it would leave the field, turning.
local function step_row(me: Object, s: State): boolean
    local y = me.panel_y
    if s.down then
        y += 1
        if y > 3 then
            s.down = false
            return false
        end
    else
        y -= 1
        if y < 1 then
            s.down = true
            return false
        end
    end
    me.panel_y = y
    return true
end

-- 0x080E5310: a column on, turning at its bounds (once: a second bound leaves it where it is).
local function step_column(me: Object, s: State)
    local turned = false
    while true do
        local x = me.panel_x + (if s.outward then me.facing else -me.facing)
        if x ~= s.near and x ~= s.far then
            me.panel_x = int.u8(x)
            return
        end
        if turned then
            return
        end
        turned = true
        s.outward = not s.outward
    end
end

-- The user's new presses while dimmed hold every key of `keys` (0x080E53DC).
local function pressed(s: State, keys: { Key }): boolean
    local user = s.user :: Object
    for _, k in keys do
        if not user:dimmed_pressed(k) then
            return false
        end
    end
    return true
end

local wait: (me: Object) -> ()

-- Action 0 (0x080E52C4): on to the next panel.
local function move(me: Object)
    local s = me.state :: State
    if not step_row(me, s) then
        step_column(me, s)
    end
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND_MOVE)
    me.action = 4
    wait(me)
end

-- The lock's power-up: SearchMan's last shot.
local function bonus(me: Object)
    local searchman = me.related1 :: Object
    searchman.state.powered = true
end

-- Action 4 (0x080E5344): a while on the panel; A locks it (with B, the power-up); time out, it locks.
wait = function(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        local ticks = if s.level ~= 0 then 4 else 6
        me.timer = ticks
        me.phase = ticks
    end
    if pressed(s, { "a" }) then
        if pressed(s, { "b" }) then
            battle.play_sound(SOUND_BONUS)
            bonus(me)
        end
        s.late_b = 5
        me.action, me.phase = 8, 0
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    me.action, me.phase = if me.timer2 ~= 0 then 0 else 8, 0
end

-- Action 8 (0x080E5398): locked on (animation 1): SearchMan has the panel; B within 5 ticks powers him; 50 ticks.
local function lock(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.anim = 1
        battle.play_sound(SOUND_LOCK)
        -- 0x080E53E6.
        local searchman = me.related1 :: Object
        searchman.future_panel_x, searchman.future_panel_y = me.panel_x, me.panel_y
        me.timer = 0x32
        me.phase = 0x32
    end
    if s.late_b ~= 0 then
        s.late_b -= 1
        if pressed(s, { "b" }) then
            bonus(me)
        end
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.lifecycle = "destroy"
    end
end

local ACTIONS = { [0] = move, [4] = wait, [8] = lock }

-- 0x080E5298: the action, the time limit, the sprite.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("SearchMan's scope action %#x reads past its table (0x080E52B8)", me.action))
    end
    action(me)
    if me.timer2 ~= 0 then
        me.timer2 -= 1
    end
    me:update_sprite()
end

scope.kind = define.kind {
    id = "serchman/scope",
    pool = "effect",
    state = {
        level = "u8",
        sight = "bool",
        user = "object",
        down = "bool",
        outward = "bool",
        near = "u8",
        far = "u8",
        late_b = "u8",
    },
    -- 0x080E5208.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080E547C: SearchMan's scope for `searchman`, his user's presses read, of a chip of `level`.
function scope.spawn(searchman: Object, user: Object, level: number): Object?
    local o = battle.spawn(scope.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.user = user
        s.level = level
        s.sight = false
        o.alliance, o.flip = searchman.alliance, searchman.flip
        o.related1 = searchman
    end
    return o
end

return scope
