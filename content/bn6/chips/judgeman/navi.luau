--!strict
-- JudgeMan (the original's actor object #0xF, `sub_80BA708`), the navi of
-- the JudgeMan chips, brought by the navi chip controller. He appears,
-- raises his whip (./whip) and lashes the three panels ahead (paralysing);
-- then, on each panel of his side's that the other side holds and that
-- stands free, a book (./book) flies at the enemy; once every book is done,
-- his columns start coming back. See docs/engine/chips.md §3.6.29.

local navi_chips = require("../../lib/navi-chips/navi")
local book = require("./book")
local collision = require("../../rules/collision")
local panels = require("../../lib/panels")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")
local whip = require("./whip")

local judgeman = {}

-- What a chip summons him with.
export type Summon = {
    -- His books' damage (the original's first chip parameter: JudgeMan 20,
    -- JudgeMn[EX] 30, JudgeMn[SP] 40).
    book_damage: number,
}

export type State = {
    -- His whip (ExtraVars[0]).
    whip: Object?,
    -- How many books, and how many are done (ExtraVars[1], [5]).
    books: number,
    books_done: number,
    -- The books' panels, packed (his bytes from +0x68).
    book_panels: { number },
    -- The books' damage: his chip's first parameter.
    book_damage: number,
    -- The controller waiting for him.
    controller: Object?,
}

local SPRITE = asset.sprite("judgeman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_LASH = asset.sound("erase-man-2")

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080BA921

-- His columns, by side (`byte_80BA9F8`, `off_80BAA84`).
local COLUMNS = { [0] = { 5, 4, 3, 2 }, [1] = { 2, 3, 4, 5 } }

-- A panel for a book: solid, nothing on it, not reserved.
local BOOK_FLOOR, BOOK_FORBID = 0x10, 0x0F88_0080

-- The lash: paralysing (status 0x11).
local LASH_STATUS = 0x11

-- His actions.
local APPEAR, STAND, RAISE, LASH, BOOKS, WAIT_BOOKS, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18

-- `sub_80BA72C`: on his panel, on the ground (Z's whole part cleared).
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground")
    me:set_lifecycle("update")
end

-- `sub_800D618`: a panel his side owns by home that the other side holds.
local function taken(x: number, y: number, side: number): boolean
    local p = field.panel(x, y)
    return p ~= nil and p.home ~= p.alliance and p.home == side
end

-- `sub_80BA7A0`: he appears; 2 ticks; on, or away if his panel can't hold
-- him.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if not navi_chips.wait(me, 2, true) then
        return
    end
    me:set_action(if navi_chips.has_footing(me) then STAND else LEAVE)
end

-- `sub_80BA7F4`: 30 ticks.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if navi_chips.wait(me, 0x1E, true) then
        me:set_action(RAISE)
    end
end

-- `sub_80BA81C`: the whip comes out; 15 ticks.
local function raise(me: Object)
    if me.phase == 0 then
        me.anim = 6;
        (me.state :: State).whip = whip.raise(me)
    end
    if navi_chips.wait(me, 0xF, false) then
        me:set_action(LASH)
    end
end

-- `sub_80BA946`: the lash's hits on the three panels ahead (`sub_80C53A6`:
-- they run while dimmed).
local function lash_hits(me: Object)
    for i = 1, 3 do
        local box = battle.hitbox(me, {
            panel_x = me.panel_x + me.facing * i,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = sparks.elec,
            target = collision.hits_navis,
            self_type = collision.curse,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 1,
            status = LASH_STATUS,
        })
        if box then
            box.run_while_dimmed = true
        end
    end
end

-- `sub_80BA9A4`: the panels for books: in his columns, the rows his side
-- lost that stand free. How many.
local function book_panels(me: Object): number
    local s = me.state :: State
    local n = 0
    for _, x in COLUMNS[me.alliance] do
        for y = 1, 3 do
            if taken(x, y, me.alliance) and field.check(x, y, BOOK_FLOOR, BOOK_FORBID) then
                s.book_panels[n + 1] = panels.pack(x, y)
                n += 1
            end
        end
    end
    return n
end

-- `sub_80BA84C`: the lash; 30 ticks; the whip goes, and on to the books if
-- there are panels for them, else he leaves.
local function lash(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        me.anim = 7
        lash_hits(me)
        battle.play_sound(SOUND_LASH)
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    whip.take_off(s.whip)
    s.books = book_panels(me)
    me:set_action(if s.books ~= 0 then BOOKS else LEAVE)
end

-- `sub_80BAA0C`: a book on each panel, dealing his chip's first parameter
-- with his damage word's flag bits and counter byte, Null; its Z is those
-- flag bits.
local function place_books(me: Object)
    local s = me.state :: State
    local flags = bit32.band(me.damage, 0xF000)
    local damage = bit32.bor(bit32.lshift(me.stamina, 16), s.book_damage, flags)
    local i = 0
    repeat
        local p = s.book_panels[i + 1]
        book.place(me, panels.x(p), panels.y(p), i, 0, damage, flags)
        i += 1
    until i >= s.books
end

-- `sub_80BA89C`: 30 ticks; the books at 15.
local function books(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 5
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0xF then
        place_books(me)
        return
    end
    if left <= 0 then
        me:set_action(WAIT_BOOKS)
    end
end

-- `sub_80BAA4C`: in each of his columns, the first row his side lost
-- starts coming back (`object_setPanelAllianceTimerShort`).
local function give_back(me: Object)
    for _, x in COLUMNS[me.alliance] do
        for y = 1, 3 do
            if taken(x, y, me.alliance) then
                field.set_column_timer(x, 1)
                break
            end
        end
    end
end

-- `sub_80BA8D0`: once every book is done, his columns come back.
local function wait_books(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0
    end
    local s = me.state :: State
    if s.books_done ~= s.books then
        return
    end
    give_back(me)
    me:set_action(LEAVE)
end

-- `sub_80BA8F6`: 2 ticks, then he goes and the controller moves on.
local function leave(me: Object)
    if me.phase == 0 then
        me.anim = 4
    end
    if not navi_chips.wait(me, 2, false) then
        return
    end
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS =
    { [APPEAR] = appear, [STAND] = stand, [RAISE] = raise, [LASH] = lash, [BOOKS] = books, [WAIT_BOOKS] = wait_books, [LEAVE] = leave }

judgeman.kind = define.kind {
    id = "judgeman/navi",
    pool = "actor",
    state = {
        whip = "object",
        books = "u8",
        books_done = "u8",
        book_panels = "u8[12]",
        book_damage = "u8",
        controller = "object",
    },
    -- `sub_80BA708`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("JudgeMan action %#x reads past his table (off_80BA784)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- A chip's `navi` hook (`sub_80BA920`, `off_802CD5C[15]`): JudgeMan for
-- the user, on the panel the controller picked, reporting to `controller`.
function judgeman.summon(args: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(judgeman.kind, user, spec, SPAWNER_ADDRESS)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.book_damage = args.book_damage
        return o
    end
end

return judgeman
