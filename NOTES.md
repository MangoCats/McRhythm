# AI tools perspective

**ALWAYS** review the AI's output, mistakes and agent misunderstandings are common.

If you're not happy with the output from your agent, describe the nature of the issues to it.  Prompt it how to do better.  Give it "good" patterns and examples to follow.  Even identical prompts on fresh contexts get varying answers, so giving more thorough / multiple prompts will tend to bring results into better alignment with your expectations, when the prompts adequately convey what you are expecting.

Having said that, it can also be efficient at times to give under-specified prompts, open questions, and see what the agent comes back with.  When the agent comes back with something you would like to develop further, it seems easier to follow those paths which it finds for itself without a lot of redirection prompting, as compared with trying to "force" a particular behavior through multiple redirection prompts.

I have noticed that some redirection prompts have a temporary effect with an agent, it follows the redirection for a while, then later reverts back to their earlier behavior.  The .cursorrules and similar configuration files are frequently re-read by the agents, so prompts saved there have more "sticky" effect - but also consume more context window...  There are also "workflow" instructions which are replayed into the agent when you invoke the / command of the workflow - these are good for initiating specific tasks, like planning a significant code refactoring.

### Productivity: A Nuanced Picture

AI-assisted coding productivity is highly context-dependent.  While [85% of developers](https://blog.jetbrains.com/research/2025/10/state-of-developer-ecosystem-2025/) now use AI tools and 88% save at least 1hr/week (JetBrains 2025), the actual gains vary dramatically:

- **Greenfield/simple projects**: 30-40% faster
- **Brownfield/complex projects**: 0-15% faster (often lower)
- **Experienced developers on familiar codebases**: May actually be [19% slower](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/) (METR RCT, 2025)

Net productivity typically averages 15-20% after accounting for error correction.  Only 31% of developers feel AI actually makes them more productive, and trust in AI-generated code has dropped from 40% to 29% year-over-year.  The real value often comes from reducing cognitive load on tedious tasks rather than raw speed improvements.

-----

## Workflows vs Tools

Synopsis: workflows are flexible and adaptable and less reliable than tools.  Tools scale to handle larger datasets and have deterministic repeatable output, but are inherently rigid - can be brittle.  Tools can be maintained and expanded, but can also balloon into meta-development projects.  Workflows can be powerful, but must be monitored carefully for appropriate output.

### Workflows

Something the AI/LLM agents have defined to make interaction easier is the concept of "workflows" which are, essentially, a set of natural language prompts that capture commonly repeated prompts for certain "workflow" scenarios.  Like planning a refactoring, or researching an architectural design decision... These workflows are non-deterministic, flexible regarding their input data sources, and can be quite powerful, but at the same time are less than 100% reliable.  As an example, for a home project I asked Claude to extract a list of line segments and arcs from a floor plan description, which it did as a workflow.  The input is a well defined, but varied .scad file, and the output is a table of about 30 segments with about 6 parameters each.  In that table, the LLM based workflow made about 6 mistakes which had to be pointed out for it to correct.  Unfortunately, no matter how "rigorous" the workflow prompts are, the output is always a little variable and never quite 100% right on the first pass.  Unlike deterministic processes, different mistakes show up on different executions of the workflow.  The workflow can be improved, for instance: checking that all the arc sweeps add up to 360 degrees, and checking that the vectors define a closed loop, but somehow it always seems to accumulate more mistakes as the job gets larger.

### Quantifying Workflow Unreliability (2025 Research)

Recent research provides hard numbers on workflow reliability:

- **Hallucination rates** for reasoning models (o3, o4-mini) range from [33% to 79%](https://www.techopedia.com/ai-hallucinations-rise) on knowledge tasks
- [25% of developers](https://www.qodo.ai/reports/state-of-ai-code-quality/) estimate that 1-in-5 AI-generated suggestions contain factual errors
- **Missing context** (reported by 65% of developers) is cited more often than hallucinations as the root cause of poor code quality
- Developers who experience fewer than 20% hallucinations are 2.5x more likely to merge code without reviewing it - a concerning feedback loop
- [Trust in AI-generated code dropped from 40% to 29%](https://blog.jetbrains.com/research/2025/10/state-of-developer-ecosystem-2025/) year-over-year despite adoption increasing (JetBrains 2025)

### Tools

In AI/LLM terms, Tools are specialized (deterministic) programs for specific tasks.  They can be bash or powershell scripts, python scripts, Rust applications, really anything at all.  (Sometimes existing applications like compilers, linters, command line tools like grep and sed, etc. are also referred to as tools, but more often tools refers to more customized applications.)  The AI/LLM can write and maintain these tools, with the caveat that simple works best: tools under about 1000 lines of code seem to be reasonably fast to develop, predictable / reliable in operation, maintainable, etc.  If a tool can be developed to do something deterministic, like extracting perimeter definitions from .scad files, then it will run deterministically: 100% repeatable.  This is good, until the tool proves to be "brittle" to changes in the input format or other changes - workflows are more adaptable and can succeed in adapting to varied inputs, but they are less repeatable.  The AI/LLM can maintain and expand its tools to handle new variations, but that can quickly become a whole meta-development project of its own, and the AI/LLM also tends to balloon the tool code size to handle new cases, quickly becoming large and complex enough that tool maintenance starts taking significant effort.

-----

## Compounding Engineering

[Compounding Engineering](https://github.com/EveryInc/compounding-engineering-plugin/tree/main?tab=readme-ov-file#what-is-compounding-engineering) is a nice [idealistic philosophy](https://every.to/source-code/my-ai-had-already-fixed-the-code-before-i-saw-it) stating: **Each unit of engineering work should make subsequent units of work easier—not harder.**  At a high level it is described as a: Plan - Implement - Review process, which seems to fit well with the strengths and weaknesses of LLM/AI agent leveraged development.

About a month ago, I developed two workflows: [/think](https://jax-mdt.visualstudio.com/_git/NIM_4.0?path=/DSP/.cursor/commands/think.md) and [/plan](https://jax-mdt.visualstudio.com/_git/NIM_4.0?path=/DSP/.cursor/commands/plan.md) to encourage Cursor to use multiple agents to execute "common" tasks during development, while also providing some aggressive guidance for context window management: keeping files small and focused, prioritizing risk reduction over effort reduction, etc.  /think was good for initial research and planning, and when a task is somewhat complex and under-defined, /think can help to make a more robust specification suitable for /plan.  /plan has been very helpful at translating specifications into detailed implementation plans which Cursor then seems relatively more successful at implementing than more interactive / free-form prompt sessions - especially for larger and more complex jobs.

Others have been working along similar lines, the Claude community has been developing a much more extensive set of workflow commands as a plugin for Claude Code, which is similar enough to Cursor that you can ask Cursor to adapt the parts (or whole) of the [Compounding Engineering plugin](https://github.com/EveryInc/compounding-engineering-plugin) you may want to try in your project(s).

### TDD with Agentic Coding

[Anthropic's best practices](https://www.anthropic.com/engineering/claude-code-best-practices) recommend test-driven development as particularly powerful with agentic coding for changes that are easily verifiable:

1. Ask agent to write tests based on expected input/output pairs
   - Be explicit: "this is TDD, do not create mock implementations"
   - Tell it to run tests and confirm they fail
2. Ask agent to write code to pass the tests
   - Explicit instruction: "do not modify the tests"
   - Tell it to keep going until all tests pass (usually takes a few iterations)
3. **Separate coding from verification** - have one agent write code while another reviews/tests it

This separation of concerns leads to higher-quality results and aligns with the Plan-Implement-Review philosophy.

### Compounding Caveats

I have not, yet, tried working with the Compounding Engineering plugin itself.  It's very new, and looking through it, it feels perhaps overly optimistic about the reliability of some of the workflows contained therein - particularly the parts about parallel development in multiple git worktrees.  Never forget the Review stage, and exercise conservative caution with frequent git commits, especially around "successful" points in the development process, to enable unwinding change branches that run down a path to undesirable results.

The other challenge I see with the compounding plugin's approach is: maintenance and organization of documentation.  It's a great idea: "every time we create a plan it becomes valuable documentation..." and that's true, but the documentation is only valuable if it is read / remembered when it is needed, and while human "context windows" are finite, LLM/AI agent's are even more limited.  A thought I just had regarding this is: assignment of an agent to review documentation and report back the important information / references for the topic at hand, instead of assigning your /plan agent to read all the documentation, have the /plan agent delegate to a document research specialist agent that does the reading for it, reducing the number of tokens /plan uses reading through irrelevant content.

The compounding plugin has defined a [/release-docs](https://github.com/EveryInc/compounding-engineering-plugin/blob/main/plugins/compounding-engineering/commands/release-docs.md) command (aka [Workflow](#workflows) ) to help organize and update the documentation, but my experience with those kind of non-deterministic command workflows is 1) they are less than 100% reliable to start with, and 2) as the volume of documentation increases (which it does rapidly with AI/LLMs doing the writing), non-deterministic command workflows tend to skip more and more of the content they are reviewing, missing more problems, making more mistakes.

-----

## Design Patterns and Cognitive Load

One of the most appealing statements I read in [Compounding Engineering](https://github.com/EveryInc/compounding-engineering-plugin?tab=readme-ov-file#why-this-makes-development-compound) is: *"Every `/compounding-engineering:plan` you create documents patterns that inform the next plan."*  The benefits of reducing cognitive load with (the 30+ year old concept of) reusable [design patterns](https://refactoring.guru/design-patterns/) are important enough to warrant repeating them here:

Design patterns reduce cognitive load by providing **pre-solved solutions** to recurring problems. When a developer (human or LLM) recognizes a pattern, they skip the effort of inventing a solution from scratch.
 
**Shared vocabulary.** Patterns enable compressed communication. "Use a Repository pattern" conveys hours of design decisions in four words. This shared language reduces the cognitive burden of explaining and understanding code organization, saving context window capacity.
 
**Predictable structure.** Patterns impose consistent organization. When code follows established conventions, developers navigate unfamiliar codebases faster.  Models transfer between projects.
 
**Reduced decision fatigue.** Every design decision consumes cognitive resources. Patterns eliminate low-value decisions by providing sensible defaults, conserving time and effort for genuinely novel problems.
 
**Risk mitigation through precedent.** Patterns represent battle-tested solutions. Using proven approaches reduces issues with hidden edge cases, freeing capacity for domain-specific concerns.
 
The key insight: patterns trade the one-time cost of learning conventions for the repeated benefit of recognition-based problem solving. They shift effort from _how to structure_ toward _what to build_.  Like the old Qt slogan: create more, code less.

-----

## Cognitive Offloading and Skill Atrophy

A significant body of 2025 research warns about the risks of over-reliance on AI coding assistants.

### The Gerlich Study (2025)

A [study by Michael Gerlich at SBS Swiss Business School](https://phys.org/news/2025-01-ai-linked-eroding-critical-skills.html) found that increased reliance on AI tools is linked to diminished critical thinking abilities:

- **Cognitive offloading correlated +0.72 with AI tool usage**
- **Cognitive offloading inversely correlated -0.75 with critical thinking**
- Younger participants (17-25) showed higher AI dependence and lower critical thinking scores
- Advanced education correlated positively with critical thinking, suggesting education mitigates some cognitive impacts

### Microsoft & Carnegie Mellon Research (2025)

[Research from Microsoft and Carnegie Mellon](https://addyo.substack.com/p/avoiding-skill-atrophy-in-the-age) found that the more people leaned on AI tools, the less critical thinking they engaged in, making it harder to summon those skills when needed.

### The Scaffold vs. Substitute Framework

The critical distinction is whether AI operates as a **scaffold** or a **substitute**:

| Scaffold | Substitute |
|----------|------------|
| Temporary, adaptable, empowering | Permanent, dependency-creating |
| Goal: strengthen internal capacity | Technology assumes responsibility |
| Need AI less over time | Skills diminish over time |

### Warning Signs for Developers

- Skipping the debugger and going straight to AI for every exception
- Not reading error messages fully before sending them to AI
- Finding it arduous to step through code or read a stacktrace
- At a loss when AI isn't available or is stumped

### Recommendations

- AI should complement rather than replace human reasoning
- Distinguish which skills are safe to offload vs. essential to keep sharp
- Losing the knack for manual memory management is one thing; losing the ability to debug a live system in an emergency because you've only ever followed AI's lead is another

-----

## When NOT to Use AI

Based on the [METR study (July 2025)](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/) and other productivity research, AI assistance may actually slow you down in certain contexts:

### Contexts Where AI May Slow Experienced Developers

| Context | Expected Net Gain | Notes |
|---------|------------------|-------|
| Familiar codebases (5+ years experience) | **-19%** (slower) | METR RCT finding |
| High complexity / Brownfield | 0-10% | Error correction cancels gains |
| Low complexity / Brownfield | 15-20% | Moderate gains |
| Low complexity / Greenfield | 30-40% | Highest gains |
| High complexity / Greenfield | 10-15% | Modest gains |

### When to Skip AI Assistance

- **Debugging emergencies** where you need preserved diagnostic skills
- **Architecture decisions** requiring deep domain reasoning
- **Familiar code** you've worked on for years (you likely know it better than the AI can infer)
- **Simple changes** where context-loading overhead exceeds task complexity
- **Security-critical code** where hallucination risk is unacceptable

### The Productivity Reality Check

A [rigorous randomized controlled trial](https://arxiv.org/abs/2507.09089) with 16 experienced developers completing 246 tasks found:
- Developers **predicted** AI would save 24% time
- Experts predicted 38-39% savings
- **Actual result**: 19% slower with AI tools

This was specifically for **experienced developers on their own codebases** - the exact scenario where we might expect AI to help most.  The study notes AI tools may still be useful for less experienced developers or unfamiliar codebases.

-----

## Legible Software

An [academic paper](https://arxiv.org/html/2508.14511v2#S3) was recently published expressing something I have been feeling while working with AI for code generation, keep your modules small, keep them clearly defined, clearly separated, well defined interfaces, and don't let the AI get overwhelmed with details while working on one of the modules.  [The Register article](https://www.theregister.com/2025/11/07/researchers_detail_legible_software_model/) about the paper is a little easier to read.  You can even have your AI agent read the article as an architectural guide before starting an implementation.

Above all, keep the operation of your software visible - whether that's through log files, or graphs of data as it gets processed, or other interfaces that clearly illustrate the current state and state changes... keep all the important processes, especially the intermediate ones that users may not usually see, as highly visible as is practical.  Log files that the agent can read for itself are super-helpful in allowing the agent to do its own debugging - and you can read along to point out where it's hallucinating.

Worth mentioning: Cursor (based on VS Code) can open projects remotely, via SSH.  So, for instance, Cursor can be running on your corporate laptop while building / debugging an application directly on the NIM Console, using native tools on the NIM Console including compilers, profilers, reading /var/nim4 log files, system log files via journalctl, other files that may be written while your application under development runs, running valgrind on your code, etc.  These are all "windows into the operation of your application" which you can instruct Cursor to use.

A great "legibility" tool for Rust applications is the [tracing](https://docs.rs/tracing/latest/tracing/) crate which can be configured to output logs very much like the NIM Vital's LogKeeper logs with: timestamps, filename and line number where the message came from, thread id, etc.  All that metadata makes the logs much more "legible" to an AI Agent, enabling them to spot unexpected or problematic behavior and accurately diagnose and fix the causes.

-----

## Specification as Code

In the recent months (July-October 2025), I have come to this same conclusion regarding programming using AI tools, luckily a slick presenter put together just about exactly what I would say if I were giving a talk on how to use AI for software creation: [Specification as Code](https://youtu.be/8rABwKRsec4?si=7JhAr_klMcKp97Yt&t=47)

### Research Support for Formal Specifications

Recent academic research strongly supports the specification-first approach:

**[SpecGen](https://arxiv.org/abs/2401.08807):** A technique for automated formal program specification generation using LLMs. It succeeded in generating verifiable specifications for 279 out of 385 programs, outperforming conventional tools like Houdini and Daikon.

**[Astrogator](https://arxiv.org/html/2507.13290):** A system using a Formal Query Language that represents user intent in a formally defined but natural language-like manner. On a benchmark of 21 LLM code-generation tasks:
- Formally proved correctness of generated code in **83% of cases**
- Precisely identified incorrect LLM generated code in **92% of cases**

**Gap in Current Practice:** There is a [clear gap in evaluations](https://arxiv.org/html/2510.03862v1) that employ formal specifications such as preconditions and postconditions. Natural language alone may not be sufficiently precise for high-quality code synthesis, and relatively few studies have examined the explicit use of formal software engineering constraints to guide code generation directly.

### Code was just another form of specification

[This article](https://generativeai.pub/the-eternal-return-of-abstraction-why-programming-was-never-about-code-18412033b517) tries hard to be entertaining, maybe succeeds at times, while recounting the evolution of specifications, code, graphic interfaces, and LLMs.  What it points to, but does not say very directly, is the importance of mature software development processes.

### Process Matters

Whether it's keeping the unreliable output of LLM coding agents in a useful state, or dealing with human developers' non-zero hallucination rates, traditional software development practices have been developed for decades around the core goals of delivering high quality software in the real world of less than perfect communication, less than perfect understanding, and less than ideal implementations.  [This study](https://arxiv.org/html/2509.13942v1) found that Agile methods (as compared with Waterfall and V model) produced better quality code for a given level of effort using LLMs.  I feel that the same domain specific processes that are applied to traditional software development can be applied with the same benefits to software development assisted by LLM agents.

[This study](/.attachments/6416-688cadef-92cb-44d5-849a-51575a21d8d9.pdf) concludes: "Our findings strongly support the hypothesis that incorporating explicit software requirements as an intermediate step between basic prompt and code generation significantly enhances output quality across both functional and non-functional dimensions. Models guided by formal specifications consistently produced more robust, maintainable, and feature-rich artifacts, often directly addressing limitations or possible enhancements identified in their default outputs."  And I would recommend taking that beyond specifications onward through the entire test, documentation and maintenance lifecycle, with special emphasis on [visible software operation](#legible-software) to assist the more important than ever humans in the loop in their evaluations of what the software is doing.

In other words: just because you're using an AI/LLM agent as a software development tool is no reason whatsoever to forget current best practices, it's actually a reason to apply those practices more rigorously / more frequently - to deal with the firehose of output that comes from an AI/LLM agent as compared with a human developer.

-----

## Cursor Buzz-words worth using

While you can converse with LLMs such as Claude-Sonnet-4.5 via Cursor, or Claude Code, or a conversational chat interface in "natural language" - I have found AI to use and understand certain terms/acronyms more frequently/naturally than others.  These are a few that have come up a lot in my time with Cursor:

### DRY

Don't Repeat Yourself

This is well understood by the agents as shorthand for: define concepts in one place and reference them as needed.  In documentation it means keep the definition in one place and reference / point to it when needed rather than copy-paste repeat / expand / refine / drift / corrupt that same concept in other parts of a project's documentation.  AI agents struggle to respect the DRY concept in documentation, but they can be re-directed / periodically instructed to "DRY out" a set of documents and reduce repetition which can be especially problematic when the core concept itself is changed and scattered copies of the old concept are not updated.  A classic example is: default values for a parameter.  AI will cheerfully copy the default value from one place to many places in the documentation and from there as a hard coded value many places in the code.  Then, you decide you want a different default value, but due to the scattered copies the implementation only changes to your new value in some places.  Review new documents and remind your agents to keep them DRY.

Coding agents understand the DRY concept well when directed to review code for DRY opportunities, and they're good at refactoring code to use DRY principles to reduce complexity and line counts, but somehow they still write a lot of new code that needs this DRY review / rework even when DRY is included as a coding standard in root instruction files like .cursorrules.  Keep your documentation DRY and keep your code DRY-er - leading to the next valuable buzzword:

### SSOT

Single Source Of Truth

Coding agents also tend toward local copies of values, buffers and caches when they really should be referencing a Single Source Of Truth.  Clearly identifying the SSOT for each concept in requirements, architecture, specifications and implementation documents is a good start, but that doesn't mean coding agents always respect those directives, they also need periodic reminders to respect and use SSOTs and refactor code which does not.

### MVP

Minimum Viable Product

When MVP is what you are after, this is a good thing.  Agents seem to automatically tend to MVP solutions, cutting out "un-necessary" features to speed implementation of a working program.  Unfortunately, even when specifically instructed that the current project is NOT AN MVP SOLUTION ***THIS IS A FULL IMPLEMENTATION OF ALL SPECIFIED FEATURES***, I still find agents "trimming" development plans and self-declaring harder to develop features as "un-necessary because..." They will cheerfully present a detailed implementation plan as "100% Complete 🎉🎉🎉***MISSION ACCOMPLISHED***🎉🎉🎉" with rows full of ✅ with no ❌ or ⚠️ to be seen, but if you dig into the 46 pages of associated documentation generated you will find quiet little agent self-directed scope reduction rationale statements which "notified the team" that aspects of the approved development plan have been "rescheduled for implementation in a later phase."

### Technical Debt

Technical debt takes many forms, from TODO and similar comments in the code flagging un- and under-implemented aspects of requirements or goals, it can be "stubbed functions" which allow the rest of the program to function without actually implementing the intended (and frequently specified or required) functionality.  Unimplmented API elements, unimplemented aspects of database schema, console log messages may show WARN or ERROR during startup or normal operation flagging significant concerns, but "all unit and integration tests are passing."  Technical debt also includes requirements lacking units tests (or, more frequently, lacking effective unit tests) un- and under-documented functions, structures, enums, etc. as well as system documentation (requirements, architecture, specifications, implementation documents) which is not synchronized with the implementation - either the documentation has evolved and code is not updated to match, or decisions were made during implementation which should be reflected in updated documentation but the documentation updates are not performed.

Agents declare development plans 100% complete, provide long detailed summary status reports stating "everything is done, fully implemented, production-ready" but giving them the prompt "review for technical debt" will almost always reveal significant shortcomings in the implementation if the development plan had a scope that required more than one "session" or any "compacting conversation" events.

### The Additive Approach

Agents tend to take an **additive approach** (add new code, expand documentation) instead of **replacement approach** (replace old functions and outdated documentation).  While this is "safer" it is also frequently frustrating when intended changes are not made and functionality reverts to old behavior that you are trying to improve / replace.  It's also problematic with limited context windows which get filled with "the old stuff" and lose sight of the new goals.

### Context Engineering (Buzzword)

The 2025 evolution beyond "prompt engineering" - curating optimal token sets across the entire inference lifecycle, not just crafting prompts.  Encompasses strategies like observation masking, context pruning, token budgeting, and RAG.  See the [dedicated section below](#context-engineering) for details.

### Agentic Coding

Now a distinct practice with [established principles](https://agentic-coding.github.io/) and [benchmarks](https://arxiv.org/html/2507.02825v1).  Refers to AI agents that autonomously write, test, and iterate on code with minimal human intervention.  Key practices include TDD workflows, verification separation, and reproducible environments.

### A2A (Agent2Agent)

Google's protocol for agent interoperability, addressing the fragmentation across different agent frameworks.  Allows agents built on different platforms to collaborate on tasks.

### MCP (Model Context Protocol)

Standardized way for agents to connect to tools dynamically.  Enables agents to discover and use tools at runtime rather than having all capabilities hardcoded.  Key feature of the [Microsoft Agent Framework](https://azure.microsoft.com/en-us/blog/introducing-microsoft-agent-framework/).

-----

## Context Engineering

After years of "prompt engineering" being the focus, 2025 has seen the emergence of **context engineering** as a distinct discipline. Building with language models is becoming less about finding the right words and more about answering: "what configuration of context is most likely to generate the model's desired behavior?" ([Anthropic](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents))

### Context Rot and Attention Budget

Studies on needle-in-a-haystack benchmarks have uncovered **"context rot"**: as tokens in the context window increase, the model's ability to accurately recall information decreases. This happens across all models, though some degrade more gracefully than others ([JetBrains Research, NeurIPS 2025](https://blog.jetbrains.com/research/2025/12/efficient-context-management/)).

Think of it this way: LLMs have an **"attention budget"** they draw on when parsing context, similar to how humans have limited working memory capacity. Context must be treated as a finite resource with diminishing marginal returns.

### Key Context Management Strategies

1. **Observation Masking**: Target environment observations while preserving action and reasoning history in full
2. **Context Pruning**: Drop oldest messages or remove verbose reasoning/tool logs after they've served their purpose
3. **LLM Summarization**: Compress long history into compact form (reduces resolution of all three parts: reasoning, action, observation)
4. **Token Budgeting**: Explicit operational practice with provenance tags, versioning, observability (logging which context was supplied), and context regression tests
5. **RAG**: Retrieval-augmented generation for dynamic context loading

### Practical Context Hygiene

When you start a new agent, you have a "clean" context initialized to whatever default state you have defined in your .cursorrules and similar customization files.  From there you can either start a prompt session (conversation) straight away, or have the agent read one or more documents to come up to speed on the topic before starting.  One good practice is to take time out during a session to have the agent write a summary document of the important things that have been recently discussed, then in later agent sessions you can have the agent read one or more of those summary documents to come up to speed for a new development session.

What I have noticed is, even when the prompt is something like "create a high level design document describing the whole system" if you are deep in a session, the document created is going to have a lot of detail about things from the session (context) and not so much on the bigger picture that the prompt was instructing the agent to write about.

So, bear this in mind when asking an agent to do... anything.  I have noticed that even after a bug is found and correct implementation has been identified, a "bad context" will sometimes repeat the buggy implementation pattern where a fresh context wouldn't have.  This is now understood as a systematic phenomenon of context rot, not just anecdotal observation.

-----

## Markdown Format

Several sources have pointed to markdown formatted documents (.md) as one of the most efficient ways to communicate with agents.  The straightforward formatting of text "tokens" in .md documents communicates more directly to the models, using fewer context tokens to convey the same concepts, especially as compared to .xml and other more complex formats.

## Research Agents and Multi-Agent Systems

Having said that, agents *can* read and "understand" (tokenize) a wide variety of file formats.  One way to optimize context focus is to use [multi-agent strategies](https://devops.com/cursor-2-0-brings-faster-ai-coding-and-multi-agent-workflows/): have some agents read and summarize low info-of-interest density documents for a primary agent to then read the summaries and synthesize from there.  The "research agents" could be reading websites, or even collections of local .md documents, anything that has a lot of distracting content that can be stripped away to keep the primary agent's context focused.  Just remember: the summarizers can and will miss important details from time to time, and the primary agent's capture of important concepts from the summaries will also be less than perfect.

### 2025 Multi-Agent Frameworks

The first quarter of 2025 marked intense activity in multi-agent AI systems, with major companies unveiling new agent platforms:

**[MetaGPT/MGX](https://github.com/FoundationAgents/MetaGPT):** "The world's first AI agent development team" - takes a one-line requirement and outputs user stories, competitive analysis, requirements, data structures, APIs, and documents.  Internally includes product managers, architects, project managers, and engineers with orchestrated SOPs.  Their paper "AFlow: Automating Agentic Workflow Generation" was accepted for oral presentation (top 1.8%) at ICLR 2025.

**[Microsoft Agent Framework](https://azure.microsoft.com/en-us/blog/introducing-microsoft-agent-framework/):** Converges AutoGen and Semantic Kernel into a unified, commercial-grade framework.  Features:
- Any API integration via OpenAPI
- Agent2Agent (A2A) cross-runtime collaboration
- Model Context Protocol (MCP) for dynamic tool connections
- Magentic One patterns for workflow orchestration

**[IBM Perspective](https://www.ibm.com/think/insights/ai-agents-2025-expectations-vs-reality):** "A bigger model as orchestrator, and smaller models doing constrained tasks."  As individual agents get more capable, there may be a shift toward single-agent systems that can handle tasks end-to-end.

### Orchestration Patterns

The emerging pattern for complex tasks:
1. **Orchestrator agent** (larger model) - task decomposition, delegation, synthesis
2. **Specialist agents** (smaller models) - constrained subtasks with focused context
3. **Research agents** - document retrieval and summarization
4. **Verification agents** - code review, testing, fact-checking

This aligns with the earlier observation about having /plan delegate to document research specialist agents rather than loading all documentation into a single context.