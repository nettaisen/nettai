--!strict
-- SearchSoul's reveal (EXE5's effect object 0x8F, 0x080EABCC: EXE6's `sub_80EA258`, the same code, which EXE6's
-- netbattles never reach), spawned by the status reset in SearchSoul (0x080EACB6): it marks each navi of the other
-- side that is invisible or flashing with the scope (effect 0x68, 20 pixels up), and on the next tick shows them
-- again: their flash timer ended, no longer invisible or flashing. Then it goes.

local reveal = {}

export type State = {
    -- ExtraVars: the navis it marked.
    marked: { Object? },
}

-- The scope's mark (effect 0x68: sprite 10-23), 20 pixels up.
local MARK = define.effect { sprite = asset.sprite("search-scope"), anim = 0, palette = 0 }
local MARK_Z = 20 * 0x1_0000

-- 0x080EAC18: the other side's hidden navis marked.
local function mark(me: Object)
    local s = me.state :: State
    local n = 0
    for _, navi in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
        if navi:status("invisible") or navi:status("flashing") then
            local e = battle.effect(navi.pos + Vec3.new(0, 0, MARK_Z), MARK)
            if e then
                e.run_while_paused = true
            end
            n += 1
            s.marked[n] = navi
        end
    end
    me.timer = 10
    me.action, me.phase = 4, 4
end

-- 0x080EAC92: the marked navis shown again; then it goes.
local function show(me: Object)
    local s = me.state :: State
    for i = 1, 4 do
        local navi = s.marked[i]
        if navi == nil then
            break
        end
        navi:set_status_timer("flash", 0)
        navi:set_status("invisible", false)
        navi:set_status("flashing", false)
    end
    me:set_lifecycle("destroy")
end

reveal.kind = define.kind {
    id = "searchsoul/reveal",
    pool = "effect",
    state = { marked = "object[4]" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080EABEC
            me:set_lifecycle("update")
        elseif l == "update" then
            if me.action == 0 then
                if me.phase_init == 0 then
                    mark(me)
                else
                    -- (Not reached: the marks leave the phase uninitialized but the action 4, which the next tick
                    -- runs.)
                    local t = me.timer - 1
                    me.timer = bit32.band(t, 0xFFFF)
                    if t < 0 then
                        me.action, me.phase, me.phase_init = 0, 0, 0
                    end
                end
            elseif me.action == 4 then
                show(me)
            else
                error(string.format("SearchSoul's reveal's action %#x reads past its table (0x080EAC10)", me.action))
            end
        else
            me:free()
        end
    end,
}

-- 0x080EACB6: the reveal for `navi`'s side, running while paused.
function reveal.spawn(navi: Object): Object?
    local o = battle.spawn(reveal.kind)
    if o then
        o.alliance = navi.alliance
        o.run_while_paused = true
    end
    return o
end

return reveal
