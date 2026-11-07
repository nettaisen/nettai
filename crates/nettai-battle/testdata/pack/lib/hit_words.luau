--!strict
-- A kind that tries the hit words' names on its update and keeps what it found in its state: a hit modifier and a
-- damage word built by name, read back by name, and what is refused (a name its set hasn't, a value that is no flag,
-- a list of names, a part a word hasn't).

local hit_words = {}

local function refused(f: () -> any): boolean
    return not pcall(f)
end

hit_words.kind = new.kind {
    id = "test/hit-words",
    pool = Pool.effect,
    state = {
        modifier = schema.u8,
        word = schema.u32,
        drag = schema.bool,
        double = schema.bool,
        -- What is refused: a hit modifier bit its set hasn't, a value that is no flag, a list of names, a name its view
        -- hasn't; a part a word hasn't, a flag its game hasn't, a counter strength past 127.
        unknown_bit = schema.bool,
        not_a_flag = schema.bool,
        a_list = schema.bool,
        unknown_view = schema.bool,
        unknown_part = schema.bool,
        unknown_flag = schema.bool,
        counter_past = schema.bool,
    },
    update = function(me: Object)
        local s = me.state :: any
        s.modifier = hit_modifier.byte({ flinch = true, flash = false, back = true, drag = true })
        s.word = damage_word.of { amount = 40, flags = { double = true, paralysis = true }, counter = 20, cant_counter = true }
        s.drag = hit_modifier.bits(s.modifier).drag
        s.double = damage_word.flags(s.word).double
        local nope: any = { shove = true }
        s.unknown_bit = refused(function()
            return hit_modifier.byte(nope)
        end)
        local one: any = { flinch = 1 }
        s.not_a_flag = refused(function()
            return hit_modifier.byte(one)
        end)
        local list: any = { "flinch" }
        s.a_list = refused(function()
            return hit_modifier.byte(list)
        end)
        s.unknown_view = refused(function()
            return (hit_modifier.bits(s.modifier) :: any).shove
        end)
        local stamina: any = { amount = 1, stamina = 2 }
        s.unknown_part = refused(function()
            return damage_word.of(stamina)
        end)
        s.unknown_flag = refused(function()
            return damage_word.of { flags = { stun = true } }
        end)
        s.counter_past = refused(function()
            return damage_word.of { counter = 0x80 }
        end)
    end,
}

return hit_words
