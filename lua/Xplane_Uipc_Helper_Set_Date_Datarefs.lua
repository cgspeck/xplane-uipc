local year = os.date("*t").year

DataRef("zuluHour", "sim/cockpit2/clock_timer/zulu_time_hours")
DataRef("zuluDOY", "sim/time/zulu_date_days")  -- 0-indexed
DataRef("localHour", "sim/cockpit2/clock_timer/local_time_hours")
DataRef("localDOY", "sim/time/local_date_days")  -- 0-indexed

define_shared_DataRef("xplane-uipc-helper/time/offset_minutes", "Int")
define_shared_DataRef("xplane-uipc-helper/time/offset_minutes_negated", "Int")
define_shared_DataRef("xplane-uipc-helper/time/offset_seconds", "Int")
define_shared_DataRef("xplane-uipc-helper/time/zulu_year", "Int")
define_shared_DataRef("xplane-uipc-helper/time/zulu_day_of_month", "Int")
define_shared_DataRef("xplane-uipc-helper/time/zulu_month", "Int")
define_shared_DataRef("xplane-uipc-helper/time/local_year", "Int")
define_shared_DataRef("xplane-uipc-helper/time/local_day_of_month", "Int")
define_shared_DataRef("xplane-uipc-helper/time/local_month", "Int")

-- ── Helper: seconds between local time and UTC ────────────────────
-- Positive value = east of UTC  (e.g. Melbourne UTC+10 → 36 000 s)
local function utcOffsetSeconds()
    local now = os.time()
    return os.difftime(now, os.time(os.date("!*t", now)))
end


local function utcOffsetMinutes()
    local now    = os.time()
    local local_t = os.date("*t",  now)   -- local time table
    local utc_t   = os.date("!*t", now)   -- utc time table

    -- Reconstruct both as timestamps using os.time()
    -- (forces both through the same normalisation path)
    local local_ts = os.time(local_t)
    local utc_ts   = os.time(utc_t)

    return os.difftime(local_ts, utc_ts) / 60
end

local offsetMin = utcOffsetMinutes()
local offsetSeconds = utcOffsetSeconds()

set("xplane-uipc-helper/time/offset_minutes", offsetMin)
set("xplane-uipc-helper/time/offset_minutes_negated", offsetMin * -1)
set("xplane-uipc-helper/time/offset_seconds", offsetSeconds)

-- ── Convert year + DOY + hour → local date table ──────────────────
-- os.time() passes month=1, day=<DOY> to mktime(), which rolls the
-- day count forward through the calendar automatically.
local function localDateFromDOY(yr, doy, hour)
    local ts = os.time({ year = yr, month = 1, day = doy,
                         hour = hour, min = 0, sec = 0 })
    return os.date("*t", ts)
end

-- ── Convert year + DOY + hour → UTC date table ────────────────────
-- Build a local timestamp, then shift by the UTC offset so that
-- reading it back with os.date("!*t", …) yields the correct UTC values.
local function zuluDateFromDOY(yr, doy, hour)
    local ts = os.time({ year = yr, month = 1, day = doy,
                         hour = hour, min = 0, sec = 0 }) + offsetSeconds
    return os.date("!*t", ts)
end

local function calculate_zulu_year(local_month, zulu_month, local_year)
    -- Wrap the inline compound logic for safety
    return (local_month == 1 and zulu_month == 12) and (local_year - 1)
        or (local_month == 12 and zulu_month == 1) and (local_year + 1)
        or local_year
end

function log_local_variables()
    local level = 2 -- Level 2 reads the variables of the function that called this one
    local idx = 1
    
    logMsg("--- Current Local Variables ---")
    while true do
        local name, value = debug.getlocal(level, idx)
        if not name then break end -- No more variables at this level
        
        -- FlyWithLua uses logMsg() to write text into X-Plane's Log.txt file
        logMsg(string.format("  %s = %s", name, tostring(value)))
        idx = idx + 1
    end
    logMsg("-------------------------------")
end

log_local_variables()

function calculateAndSetDates()
    -- ── Calculate ─────────────────────────────────────────────────────
    local lDate = localDateFromDOY(year, localDOY + 1, localHour)
    local zDate = zuluDateFromDOY(year, zuluDOY + 1,   zuluHour)

    local actual_zulu_year = calculate_zulu_year(lDate.month, zDate.month, year)

    set("xplane-uipc-helper/time/zulu_year", actual_zulu_year)
    set("xplane-uipc-helper/time/zulu_day_of_month", zDate.day)
    set("xplane-uipc-helper/time/zulu_month", zDate.month)

    set("xplane-uipc-helper/time/local_year", lDate.year)
    set("xplane-uipc-helper/time/local_day_of_month", lDate.day)
    set("xplane-uipc-helper/time/local_month", lDate.month)
end

do_often("calculateAndSetDates()")
