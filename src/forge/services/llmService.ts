export interface LLMConfig {
  provider: "ollama" | "openai" | "custom";
  baseUrl?: string;
  apiKey?: string;
  model: string;
}

const DEFAULT_CONFIG: LLMConfig = {
  provider: "ollama",
  baseUrl: "http://localhost:11434",
  model: "qwen2.5-coder:7b",
};

export class ForgeLLMService {
  private config: LLMConfig;

  constructor(config?: Partial<LLMConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  updateConfig(newConfig: Partial<LLMConfig>) {
    this.config = { ...this.config, ...newConfig };
  }

  getConfig(): LLMConfig {
    return this.config;
  }

  /**
   * P10.5-A1 & A2: تجزیه هوشمند پرامپت خام به ساختار دقیق TaskIntent با استفاده از LLM
   */
  async decomposeTaskIntent(
    rawPrompt: string
  ): Promise<{ goal: string; symbolHint: string; scope?: string }> {
    const systemPrompt = `You are Vora Forge Intent Compiler.
Analyze the user's natural language instruction and extract the exact target symbol name (function, struct, method, or component) and the core objective.
CRITICAL RULES:
1. Output ONLY a valid raw JSON object matching: {"goal": "...", "symbolHint": "...", "scope": "..."}
2. No markdown wrappers, no backticks, no chat explanations.`;

    const userPrompt = `Developer Instruction:
"""
${rawPrompt}
"""

Extract and respond in raw JSON:`;

    try {
      let rawResponse = "";
      if (this.config.provider === "ollama") {
        const endpoint = `${this.config.baseUrl || "http://localhost:11434"}/api/generate`;
        const res = await fetch(endpoint, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: this.config.model,
            system: systemPrompt,
            prompt: userPrompt,
            stream: false,
          }),
        });
        if (res.ok) {
          const json = await res.json();
          rawResponse = json.response || "";
        }
      } else {
        const endpoint = `${this.config.baseUrl || "https://api.openai.com/v1"}/chat/completions`;
        const res = await fetch(endpoint, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${this.config.apiKey || ""}`,
          },
          body: JSON.stringify({
            model: this.config.model || "gpt-4o-mini",
            messages: [
              { role: "system", content: systemPrompt },
              { role: "user", content: userPrompt },
            ],
            temperature: 0.1,
          }),
        });
        if (res.ok) {
          const json = await res.json();
          rawResponse = json.choices?.[0]?.message?.content || "";
        }
      }

      const cleaned = this.cleanResponse(rawResponse).replace(/```json/gi, "").replace(/```/g, "").trim();
      const parsed = JSON.parse(cleaned);
      if (parsed.symbolHint && typeof parsed.symbolHint === "string" && parsed.symbolHint.trim().length > 0) {
        return {
          goal: parsed.goal || rawPrompt,
          symbolHint: parsed.symbolHint.trim(),
          scope: parsed.scope,
        };
      }
    } catch {
      // فال‌بک ایمن در صورت عدم دسترسی به سرور مدل یا خطای تجزیه JSON
    }

    const words = rawPrompt.match(/[a-zA-Z_][a-zA-Z0-9_]+/g) || [];
    return {
      goal: rawPrompt.trim(),
      symbolHint: words[0] ?? "main",
    };
  }

  /**
   * تولید قطعه‌کد اصلاح‌شده بر مبنای قصد کاربر و نماد اصلی
   */
  async generateRefactoredSymbol(
    symbolName: string,
    originalCodeSnippet: string,
    userIntent: string,
    filePath: string
  ): Promise<string> {
    const systemPrompt = `You are Vora Forge Surgical Code Refactor Engine.
Your task is to modify the provided AST Symbol according to the user's intent.
CRITICAL RULES:
1. Output ONLY the raw valid code replacement for the target symbol.
2. DO NOT include markdown backticks (no \`\`\`rust or \`\`\`), no explanations, no chat intro.
3. Keep the exact symbol name and required interface contracts.
4. Ensure valid syntax and proper type handling.`;

    const userPrompt = `File: ${filePath}
Target Symbol: ${symbolName}

Original Code:
${originalCodeSnippet}

User Refactoring Intent:
${userIntent}

Provide the complete updated implementation of this symbol:`;

    if (this.config.provider === "ollama") {
      const endpoint = `${this.config.baseUrl || "http://localhost:11434"}/api/generate`;
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: this.config.model,
          system: systemPrompt,
          prompt: userPrompt,
          stream: false,
        }),
      });

      if (!res.ok) {
        throw new Error(`Ollama generation failed: HTTP ${res.status}`);
      }

      const json = await res.json();
      return this.cleanResponse(json.response || "");
    } else {
      // پشتیبانی از OpenAI / Generic API
      const endpoint = `${this.config.baseUrl || "https://api.openai.com/v1"}/chat/completions`;
      const res = await fetch(endpoint, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${this.config.apiKey || ""}`,
        },
        body: JSON.stringify({
          model: this.config.model || "gpt-4o-mini",
          messages: [
            { role: "system", content: systemPrompt },
            { role: "user", content: userPrompt },
          ],
          temperature: 0.1,
        }),
      });

      if (!res.ok) {
        throw new Error(`OpenAI API failed: HTTP ${res.status}`);
      }

      const json = await res.json();
      return this.cleanResponse(json.choices?.[0]?.message?.content || "");
    }
  }

  /**
   * P11.1: خودترمیمی کد کاندید پس از شکست در آزمون‌های سندباکس
   */
  async healRefactoredSymbol(
    symbolName: string,
    failingSnippet: string,
    testFailureLog: string,
    userIntent: string,
    filePath: string
  ): Promise<string> {
    const systemPrompt = `You are Vora Forge Self-Healing Code Engine.
The previous patch candidate failed compilation or unit tests in the sandbox.
Your job is to fix the errors and provide a working implementation.
CRITICAL RULES:
1. Output ONLY the raw fixed code for the target symbol.
2. DO NOT include markdown backticks, no explanations, no commentary.
3. Fix the specific compilation or assertion errors described in the failure log.`;

    const userPrompt = `File: ${filePath}
Symbol: ${symbolName}
Original Intent: ${userIntent}

Failing Candidate Code:
${failingSnippet}

Sandbox Test Failure Log:
${testFailureLog}

Provide the corrected, working implementation of '${symbolName}':`;

    if (this.config.provider === "ollama") {
      const endpoint = `${this.config.baseUrl || "http://localhost:11434"}/api/generate`;
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: this.config.model,
          system: systemPrompt,
          prompt: userPrompt,
          stream: false,
        }),
      });

      if (!res.ok) {
        throw new Error(`Self-healing generation failed: HTTP ${res.status}`);
      }

      const json = await res.json();
      return this.cleanResponse(json.response || "");
    } else {
      const endpoint = `${this.config.baseUrl || "https://api.openai.com/v1"}/chat/completions`;
      const res = await fetch(endpoint, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${this.config.apiKey || ""}`,
        },
        body: JSON.stringify({
          model: this.config.model || "gpt-4o-mini",
          messages: [
            { role: "system", content: systemPrompt },
            { role: "user", content: userPrompt },
          ],
          temperature: 0.1,
        }),
      });

      if (!res.ok) {
        throw new Error(`OpenAI Self-healing failed: HTTP ${res.status}`);
      }

      const json = await res.json();
      return this.cleanResponse(json.choices?.[0]?.message?.content || "");
    }
  }

  /**
   * تولید مستندات فنی و ساختاریافته برای نماد جهت ایجاد کارت دانشی در Mind
   */
  async generateSymbolDocumentation(
    symbolName: string,
    signature: string,
    filePath: string
  ): Promise<{ title: string; content: string }> {
    const systemPrompt = `You are a Technical Documentation Specialist.
Generate concise, high-value Markdown documentation for the provided code symbol.
Include: Summary, Parameters/Fields, and Architecture Notes.`;

    const userPrompt = `File: ${filePath}
Symbol: ${symbolName}
Signature: ${signature}`;

    let docMarkdown = "";
    if (this.config.provider === "ollama") {
      const endpoint = `${this.config.baseUrl || "http://localhost:11434"}/api/generate`;
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: this.config.model,
          system: systemPrompt,
          prompt: userPrompt,
          stream: false,
        }),
      });
      if (res.ok) {
        const json = await res.json();
        docMarkdown = json.response || "";
      }
    } else {
      const endpoint = `${this.config.baseUrl || "https://api.openai.com/v1"}/chat/completions`;
      const res = await fetch(endpoint, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${this.config.apiKey || ""}`,
        },
        body: JSON.stringify({
          model: this.config.model || "gpt-4o-mini",
          messages: [
            { role: "system", content: systemPrompt },
            { role: "user", content: userPrompt },
          ],
          temperature: 0.2,
        }),
      });
      if (res.ok) {
        const json = await res.json();
        docMarkdown = json.choices?.[0]?.message?.content || "";
      }
    }

    return {
      title: `Doc: ${symbolName}`,
      content:
        docMarkdown.trim() ||
        `## ${symbolName}\n\nDefined in \`${filePath}\`\n\n\`\`\`rust\n${signature}\n\`\`\`\n\n### Overview\nAutomated architectural documentation for symbol \`${symbolName}\`.`,
    };
  }

  private cleanResponse(raw: string): string {
    let text = raw.trim();
    if (text.startsWith("```")) {
      const firstLineEnd = text.indexOf("\n");
      if (firstLineEnd !== -1) {
        text = text.substring(firstLineEnd + 1);
      }
      if (text.endsWith("```")) {
        text = text.substring(0, text.length - 3);
      }
    }
    return text.trim();
  }
}

export const forgeLLMService = new ForgeLLMService();