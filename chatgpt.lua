local M = {}
local is_receiving = false

local gpt_cmd = os.getenv("SHELLBOT")
local separator = "==="

local roles = {
  USER = "◭🧑 " .. os.getenv('USER'),
  ASSISTANT = "◮🤖 vimbot",
}

local buffer_sync_cursor = {}
function ChatGPTCancelCursorSync()
  local bufnr = vim.api.nvim_get_current_buf()
  buffer_sync_cursor[bufnr] = false
  vim.api.nvim_buf_del_keymap(bufnr, 'n', '<Enter>')
  vim.api.nvim_buf_del_keymap(bufnr, 'n', '<Space>')
end

local function add_transcript_header(winnr, bufnr, role, line_num)
  local line = ((line_num ~= nil) and line_num) or vim.api.nvim_buf_line_count(bufnr)
  vim.api.nvim_buf_set_lines(bufnr, line, line + 1, false, { roles[role] })
  if role == "USER" and buffer_sync_cursor[bufnr] then
    vim.schedule(function()
      local is_current = winnr == vim.api.nvim_get_current_win()
      vim.api.nvim_win_call(winnr, function()
        vim.cmd("normal! Go")
        if is_current then
          vim.cmd('startinsert!')
        end
      end)
    end)
  end
  return line
end

local ChatGPTCancelJob = nil
function ChatGPTSubmit()
  if is_receiving then
    print("Already receiving")
    return
  end
  vim.cmd("normal! Go")
  local winnr = vim.api.nvim_get_current_win()
  local bufnr = vim.api.nvim_get_current_buf()
  buffer_sync_cursor[bufnr] = true
  local function receive_stream(_, data, _)
    if #data > 1 or data[1] ~= '' then
      local current_line = vim.api.nvim_buf_line_count(bufnr)
      local col = #vim.api.nvim_buf_get_lines(bufnr, current_line - 1, current_line, false)[1]

      current_line = current_line - 1
      -- print("data " .. current_line .. "," .. col)

      -- - {data}	    Raw data (|readfile()|-style list of strings) read from
      -- the channel. EOF is a single-item list: `['']`. First and
      -- last items may be partial lines! |channel-lines|
      vim.api.nvim_buf_set_option(bufnr, 'modifiable', true)
      for i, new_text in ipairs(data) do
        -- new_text = "[" .. new_text .. "]"
        -- print(i .. ": " .. new_text .. " :" .. current_line .."," .. col .. "|" .. #new_text)
        if i == 1 then
          if #new_text > 0 then
            vim.api.nvim_buf_set_text(bufnr, current_line, col, current_line, col, { new_text })
            col = col + #new_text
          end
        else
          current_line = current_line + 1
          vim.api.nvim_buf_set_lines(bufnr, current_line, current_line, false, { new_text })
          col = #new_text
        end
      end
      if buffer_sync_cursor[bufnr] then
        vim.schedule(function()
          vim.api.nvim_win_call(winnr, function()
            vim.cmd("normal! G$")
          end)
        end)
      end
    end
  end

  local is_interrupted = false
  local function stream_done()
    vim.api.nvim_buf_set_option(bufnr, 'modifiable', true)
    is_receiving = false
    if is_interrupted then
      vim.api.nvim_buf_set_lines(bufnr, -1, -1, false, { "❌ Interrupted" })
    end
    add_transcript_header(winnr, bufnr, "USER")
    is_interrupted = false
    ChatGPTCancelJob = nil
  end

  local function get_transcript()
    local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
    for i, line in ipairs(lines) do
      if line:match("^◭") then -- '^' means start of line
        lines[i] = separator .. "USER" .. separator
      elseif line:match("^◮") then
        lines[i] = separator .. "ASSISTANT" .. separator
      end
    end
    return lines
  end

  local function generate_buffer_name(user_input)
    local summary_prompt = "Your role is to summarize the topic of a user prompt " ..
      "to an AI assistant. Respond with a plain text string that summarizes the " ..
      "user input. Don't include special characters. Make your response shorter than 50 characters."
    local async_handle = vim.loop.new_async(vim.schedule_wrap(function()
      local output = {}

      local job_id = vim.fn.jobstart(gpt_cmd, {
        on_stdout = function(_, data, _)
          if data[1] ~= "" then
            table.insert(output, data[1])
          end
        end,
        on_exit = function()
          -- Process the response and set the buffer name
          local response = table.concat(output, "")
          vim.api.nvim_buf_set_name(bufnr, response)
        end
      })

      vim.fn.chansend(job_id, separator .. "SYSTEM" .. separator .. "\n")
      vim.fn.chansend(job_id, summary_prompt .. "\n")
      vim.fn.chansend(job_id, separator .. "USER" .. separator .. "\n")
      vim.fn.chansend(job_id, user_input .. "\n")
      vim.fn.chanclose(job_id, "stdin")
    end))
    async_handle:send()
  end


  local function get_user_input(transcript)
    local user_input = {}
    local is_user_input = false

    for _, line in ipairs(transcript) do
      if line == separator .. "USER" .. separator then
        is_user_input = true
      elseif line == separator .. "ASSISTANT" .. separator then
        if is_user_input then
          break
        end
      elseif is_user_input then
        table.insert(user_input, line)
      end
    end

    return table.concat(user_input, "\n")
  end

  local job_id = vim.fn.jobstart(gpt_cmd, {
    on_stdout = receive_stream,
    on_exit = stream_done,
    on_stderr = function(_, data, _)
      for _, str in ipairs(data) do
        vim.api.nvim_echo({{str, "ErrorMsg"}}, true, {})
      end
    end,
  })

  if job_id > 0 then
    ChatGPTCancelJob = function()
      is_interrupted = true
      ChatGPTCancelJob = nil
      vim.fn.jobstop(job_id)
    end
    is_receiving = true
    local transcript = get_transcript()
    -- Set the buffer name if it's unnamed
    local buf_name = vim.api.nvim_buf_get_name(bufnr)
    if buf_name == "" then
      local user_input = get_user_input(transcript)
      generate_buffer_name(user_input)
    end

    for _, line in ipairs(transcript) do
      vim.fn.chansend(job_id, line .. "\n")
      -- print(line)
    end
    local line = add_transcript_header(winnr, bufnr, "ASSISTANT")
    vim.api.nvim_buf_set_lines(bufnr, line + 1, line + 1, false, { "" })
    vim.api.nvim_buf_set_option(bufnr, 'modifiable', false)
    vim.fn.chanclose(job_id, "stdin")
    vim.api.nvim_command('stopinsert')
    vim.api.nvim_buf_set_keymap(bufnr, 'n', '<Enter>',
      ':lua ChatGPTCancelCursorSync()<cr>', { noremap = true, silent = true })
    vim.api.nvim_buf_set_keymap(bufnr, 'n', '<Space>',
      ':lua ChatGPTCancelCursorSync()<cr>', { noremap = true, silent = true })
    vim.api.nvim_buf_set_keymap(bufnr, 'n', '<C-c>',
      ':lua ChatGPTCancelResponse()<cr>', { noremap = true, silent = true })
  else
    print("Failed to start command")
  end
  if job_id == -1 then
    vim.api.nvim_echo({ { "Failed to start the command", "ErrorMsg" } }, true, {})
  end
end

function ChatGPTNewBuf()
  vim.cmd("enew")
  ChatGPTInit()
end

function ChatGPTInit()
  local winnr = vim.api.nvim_get_current_win()
  local bufnr = vim.api.nvim_get_current_buf()
  buffer_sync_cursor[bufnr] = true
  vim.wo.breakindent = true
  vim.wo.wrap = true
  vim.wo.linebreak = true
  vim.api.nvim_buf_set_option(bufnr, 'filetype', 'shellbot')
  vim.api.nvim_buf_set_option(bufnr, 'buftype', 'nofile')
  vim.api.nvim_buf_set_option(bufnr, 'buflisted', true)
  vim.api.nvim_buf_set_option(bufnr, 'modified', false)
  add_transcript_header(winnr, bufnr, "USER", 0)
  local modes = { 'n', 'i' }
  for _, mode in ipairs(modes) do
    vim.api.nvim_buf_set_keymap(bufnr, mode, '<C-Enter>', '<ESC>:lua ChatGPTSubmit()<CR>',
      { noremap = true, silent = true })
    vim.api.nvim_buf_set_keymap(bufnr, mode, '<C-o>', '<ESC>:lua ChatGPTNewBuf()<CR>',
      { noremap = true, silent = true })
  end
end

function M.chatgpt()
  vim.cmd("botright vnew")
  vim.cmd("set winfixwidth")
  vim.cmd("vertical resize 60")
  ChatGPTInit()
end

function M.chatgpt_init()
  ChatGPTInit()
end

function ChatGPTCancelResponse()
  if ChatGPTCancelJob then
    ChatGPTCancelJob()
  end
end

return M
