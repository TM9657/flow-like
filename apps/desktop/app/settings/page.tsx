"use client"
import { Button } from "@tm9657/flow-like-ui"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@tm9657/flow-like-ui"
import { Checkbox } from "@tm9657/flow-like-ui/components/ui/checkbox"
import { Input } from "@tm9657/flow-like-ui"

export default function SettingsPage() {
    return <div className="grid gap-6">
        <Card>
            <CardHeader>
                <CardTitle>Store Name</CardTitle>
                <CardDescription>
                    Used to identify your store in the marketplace.
                </CardDescription>
            </CardHeader>
            <CardContent>
                <form>
                    <Input placeholder="Store Name" />
                </form>
            </CardContent>
            <CardFooter className="border-t px-6 py-4">
                <Button>Save</Button>
            </CardFooter>
        </Card>
        <Card>
            <CardHeader>
                <CardTitle>Plugins Directory</CardTitle>
                <CardDescription>
                    The directory within your project, in which your plugins are
                    located.
                </CardDescription>
            </CardHeader>
            <CardContent>
                <form className="flex flex-col gap-4">
                    <Input
                        placeholder="Project Name"
                        defaultValue="/content/plugins"
                    />
                    <div className="flex items-center space-x-2">
                        <Checkbox id="include" defaultChecked />
                        <label
                            htmlFor="include"
                            className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
                        >
                            Allow administrators to change the directory.
                        </label>
                    </div>
                </form>
            </CardContent>
            <CardFooter className="border-t px-6 py-4">
                <Button>Save</Button>
            </CardFooter>
        </Card>
    </div>

}